// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::{Actor, AgentId};

use crate::common::errors::ApplicationError;
use crate::common::interfaces::{ActionRecorder, Clock};
use crate::common::models::{AgentAction, AgentActionEntry};

/// Writes the action record entries of the use cases. In 0001 the actor is
/// always the user.
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

    /// # Errors
    /// `Storage` when the entry cannot be written.
    pub fn record(
        &self,
        agent_id: AgentId,
        action: AgentAction,
        failure: Option<&str>,
    ) -> Result<(), ApplicationError> {
        self.recorder.record(&AgentActionEntry {
            actor: Actor::User,
            agent_id,
            action,
            failure: failure.map(str::to_owned),
            at: self.clock.now(),
        })?;
        Ok(())
    }
}
