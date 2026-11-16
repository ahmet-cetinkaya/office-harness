// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_domain::value_objects::ExitResult;
use kernel::AgentId;

use crate::common::errors::ApplicationError;
use crate::common::interfaces::{Clock, SessionStore};

/// Closes the running session of an agent when its terminal reports an exit.
/// The bridge turns terminal events into calls of this handler.
#[derive(Clone)]
pub struct EndSessionHandler {
    sessions: Arc<dyn SessionStore>,
    clock: Arc<dyn Clock>,
}

impl EndSessionHandler {
    #[must_use]
    pub fn new(sessions: Arc<dyn SessionStore>, clock: Arc<dyn Clock>) -> Self {
        Self { sessions, clock }
    }

    /// Returns false when the agent had no running session, as when a stop
    /// already closed it.
    ///
    /// # Errors
    /// `Storage`.
    pub fn handle(&self, agent_id: AgentId, result: &ExitResult) -> Result<bool, ApplicationError> {
        Ok(self
            .sessions
            .end_running(agent_id, result, self.clock.now())?)
    }
}

#[cfg(test)]
#[path = "end_session_handler.tests.rs"]
mod tests;
