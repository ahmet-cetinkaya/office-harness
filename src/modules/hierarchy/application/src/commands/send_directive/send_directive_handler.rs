// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use hierarchy_domain::{HierarchyError, check_descendant, validate_directive_text};
use kernel::AgentId;

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
        }
    }

    /// # Errors
    /// `Storage` when the tree cannot be read; `Invalid` naming the rule when
    /// the target is not a descendant, the text breaks the directive rules,
    /// or the target is not running; `Conflict` carrying the writer's own
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
            .map_err(|error| self.journaled(sender, target, error))?;
        if let Err(error) = check_descendant(&snapshot.rows, sender, target) {
            return Err(self.journaled(sender, target, error.into()));
        }
        if let Err(error) = validate_directive_text(text) {
            return Err(self.journaled(sender, target, error.into()));
        }
        if !self.targets.is_running(target) {
            return Err(self.journaled(
                sender,
                target,
                HierarchyApplicationError::Invalid(HierarchyError::NotRunning {
                    name: target.to_string(),
                }),
            ));
        }
        if let Err(message) = self.writer.write(target, text) {
            return Err(self.journaled(
                sender,
                target,
                HierarchyApplicationError::Conflict(message),
            ));
        }
        self.journal.record(sender, target, None);
        Ok(())
    }

    /// Records the attempt before the caller sees its outcome (FR-011): the
    /// action record carries every refusal, with the reason as shown.
    fn journaled(
        &self,
        sender: AgentId,
        target: AgentId,
        error: HierarchyApplicationError,
    ) -> HierarchyApplicationError {
        self.journal
            .record(sender, target, Some(&error.to_string()));
        error
    }
}

#[cfg(test)]
#[path = "send_directive_handler.tests.rs"]
mod tests;
