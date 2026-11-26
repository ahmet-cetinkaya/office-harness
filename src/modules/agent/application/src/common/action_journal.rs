// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::AgentId;

use crate::common::actor_scope::ActorScope;
use crate::common::errors::ApplicationError;
use crate::common::interfaces::{ActionRecorder, Clock};
use crate::common::models::{AgentAction, AgentActionEntry};
use crate::common::session_launcher::StartOutcome;

/// Joins two details of one entry.
const DETAIL_SEPARATOR: &str = "; ";

/// Writes the action record entries of the use cases, attributing them to the
/// actor set by [`ActorScope::enter`].
#[derive(Clone)]
pub struct ActionJournal {
    recorder: Arc<dyn ActionRecorder>,
    clock: Arc<dyn Clock>,
}

impl ActionJournal {
    #[must_use]
    pub fn new(recorder: Arc<dyn ActionRecorder>, clock: Arc<dyn Clock>) -> Self {
        Self { recorder, clock }
    }

    /// Records a start with its failure, or the refusal of the request, and
    /// with `detail` and the note of a sign-in check that gave no answer
    /// (0019/FR-004).
    ///
    /// # Errors
    /// `Storage` when the entry cannot be written.
    pub fn record_start(
        &self,
        agent_id: AgentId,
        action: AgentAction,
        outcome: &Result<StartOutcome, ApplicationError>,
        detail: Option<&str>,
    ) -> Result<(), ApplicationError> {
        let (failure, note) = match outcome {
            Ok(outcome) => (outcome.failure.clone(), outcome.sign_in_note.as_deref()),
            Err(error) => (Some(error.to_string()), None),
        };
        let detail = match (detail, note) {
            (Some(detail), Some(note)) => Some(format!("{detail}{DETAIL_SEPARATOR}{note}")),
            (detail, note) => detail.or(note).map(str::to_owned),
        };
        self.record_detailed(agent_id, action, failure.as_deref(), detail.as_deref())
    }

    /// # Errors
    /// `Storage` when the entry cannot be written.
    pub fn record(
        &self,
        agent_id: AgentId,
        action: AgentAction,
        failure: Option<&str>,
    ) -> Result<(), ApplicationError> {
        self.record_detailed(agent_id, action, failure, None)
    }

    /// Like [`Self::record`], with the reason the action happened, for an
    /// action that its name alone does not explain.
    ///
    /// # Errors
    /// `Storage` when the entry cannot be written.
    pub fn record_detailed(
        &self,
        agent_id: AgentId,
        action: AgentAction,
        failure: Option<&str>,
        detail: Option<&str>,
    ) -> Result<(), ApplicationError> {
        self.recorder.record(&AgentActionEntry {
            actor: ActorScope::current(),
            agent_id,
            action,
            failure: failure.map(str::to_owned),
            detail: detail.map(str::to_owned),
            actor_label: ActorScope::label().map(|label| label.to_string()),
            at: self.clock.now(),
        })?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "action_journal.tests.rs"]
mod tests;
