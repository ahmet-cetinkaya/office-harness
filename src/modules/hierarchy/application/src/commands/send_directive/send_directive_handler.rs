// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use hierarchy_domain::{AgentAction, HierarchyError, check_descendant, validate_directive_text};
use kernel::AgentId;

use crate::commands::hold::HoldService;
use crate::common::errors::HierarchyApplicationError;
use crate::common::interfaces::HierarchyStore;
use crate::common::ports::{DirectiveJournal, DirectiveTarget, DirectiveWriter};

/// Sends one directive from `sender` to one of its descendants (FR-009,
/// FR-011). The target must sit in the sender's subtree, the text must pass
/// the directive rules, and the target's session must be running; one writer
/// call delivers the text and the bridge appends the Enter. Every refusal is
/// journaled before the caller sees it, so the action record keeps every
/// attempt.
pub struct SendDirectiveHandler {
    store: Arc<dyn HierarchyStore>,
    targets: Arc<dyn DirectiveTarget>,
    writer: Arc<dyn DirectiveWriter>,
    journal: Arc<dyn DirectiveJournal>,
    hold_guard: Option<Arc<HoldService>>,
}

impl SendDirectiveHandler {
    #[must_use]
    pub fn new(
        store: Arc<dyn HierarchyStore>,
        targets: Arc<dyn DirectiveTarget>,
        writer: Arc<dyn DirectiveWriter>,
        journal: Arc<dyn DirectiveJournal>,
    ) -> Self {
        Self {
            store,
            targets,
            writer,
            journal,
            hold_guard: None,
        }
    }

    /// Puts the hold and rate-limit gate in front of the terminal write.
    /// The gate journals its own refusals, so the directive journal adds
    /// nothing for them (0007).
    #[must_use]
    pub fn with_hold_guard(mut self, hold_guard: Arc<HoldService>) -> Self {
        self.hold_guard = Some(hold_guard);
        self
    }

    /// # Errors
    /// `Storage` when the tree cannot be read; `Invalid` naming the rule when
    /// the target is not a descendant, the text breaks the directive rules,
    /// or the target is not running; `Refused` with the reason when a hold or
    /// a rate limit stops it; `Conflict` carrying the writer's own
    /// message when the text cannot be written.
    pub fn handle(
        &self,
        sender: AgentId,
        target: AgentId,
        text: &str,
    ) -> Result<(), HierarchyApplicationError> {
        let snapshot = self
            .store
            .snapshot()
            .map_err(|error| self.journaled(sender, target, text, error))?;
        if let Err(error) = check_descendant(&snapshot.rows, sender, target) {
            return Err(self.journaled(sender, target, text, error.into()));
        }
        if let Err(error) = validate_directive_text(text) {
            return Err(self.journaled(sender, target, text, error.into()));
        }
        // A store failure behind the lookup is `Storage`, its own message:
        // the target may run, but the office cannot say so (Efe r2 LOW).
        if !self
            .targets
            .is_running(target)
            .map_err(HierarchyApplicationError::Storage)?
        {
            let name = self
                .targets
                .name_of(target)
                .unwrap_or_else(|| target.to_string());
            return Err(self.journaled(
                sender,
                target,
                text,
                HierarchyApplicationError::Invalid(HierarchyError::NotRunning { name }),
            ));
        }
        if let Some(hold_guard) = &self.hold_guard {
            // After every rule above, so a directive the rules refuse or a
            // stopped target never counts toward a limit (spec A10).
            hold_guard
                .guard(
                    sender,
                    AgentAction::Directive { target },
                    Some(target),
                    &snapshot.rows,
                )
                .map_err(|refusal| HierarchyApplicationError::Refused(refusal.reason))?;
        }
        if let Err(message) = self.writer.write(target, text) {
            return Err(self.journaled(
                sender,
                target,
                text,
                HierarchyApplicationError::Conflict(message),
            ));
        }
        self.journal.record(sender, target, text, None);
        Ok(())
    }

    /// Records the attempt before the caller sees its outcome (FR-011): the
    /// action record carries every refusal with the text as sent and the
    /// reason, so the audit shows what was attempted.
    fn journaled(
        &self,
        sender: AgentId,
        target: AgentId,
        text: &str,
        error: HierarchyApplicationError,
    ) -> HierarchyApplicationError {
        self.journal
            .record(sender, target, text, Some(&error.to_string()));
        error
    }
}

#[cfg(test)]
#[path = "send_directive_handler.tests.rs"]
mod tests;
