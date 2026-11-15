// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::time::Duration;

use agent_domain::entities::Agent;
use agent_domain::enums::StatusEvent;
use agent_domain::value_objects::ExitResult;

use crate::common::errors::{ApplicationError, TerminalHostError};
use crate::common::interfaces::{AgentStore, Clock, SessionStore, SettingsStore, TerminalHost};

/// Ends the running session of an agent on the user's request: the terminal
/// asks, waits the grace period, then forces (A3).
#[derive(Clone)]
pub struct SessionEnder {
    agents: Arc<dyn AgentStore>,
    settings: Arc<dyn SettingsStore>,
    sessions: Arc<dyn SessionStore>,
    terminals: Arc<dyn TerminalHost>,
    clock: Arc<dyn Clock>,
}

impl SessionEnder {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        settings: Arc<dyn SettingsStore>,
        sessions: Arc<dyn SessionStore>,
        terminals: Arc<dyn TerminalHost>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            agents,
            settings,
            sessions,
            terminals,
            clock,
        }
    }

    /// Stops the child, then sets `stopped` and closes the session. A child
    /// that ended on its own a moment ago changes nothing here: its exit
    /// event sets the final status.
    ///
    /// # Errors
    /// `Internal` when the child cannot be ended, which leaves the status as
    /// it was.
    pub fn stop(&self, agent: &mut Agent) -> Result<(), ApplicationError> {
        let grace = Duration::from_millis(self.settings.load()?.stop_grace_ms);
        match self.terminals.stop(agent.id, grace) {
            Ok(()) => {}
            Err(TerminalHostError::NotRunning) => return Ok(()),
            Err(error) => return Err(ApplicationError::Internal(error.to_string())),
        }
        agent.apply(StatusEvent::StoppedByUser)?;
        self.agents.update_status(agent.id, agent.status)?;
        self.sessions
            .end_running(agent.id, &ExitResult::Signal, self.clock.now())?;
        Ok(())
    }
}
