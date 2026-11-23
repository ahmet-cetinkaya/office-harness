// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::time::Duration;

use agent_domain::entities::Agent;
use agent_domain::enums::StatusEvent;
use agent_domain::value_objects::ExitResult;

use crate::common::errors::{ApplicationError, TerminalHostError};
use crate::common::interfaces::{AgentStore, Clock, SessionStore, SettingsStore, TerminalHost};

/// Whether the terminal still had a child to end.
#[derive(Debug, PartialEq, Eq)]
enum ChildEnd {
    Stopped,
    AlreadyGone,
}

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
        match self.stop_child(agent)? {
            ChildEnd::Stopped => self.record_end(agent, StatusEvent::StoppedByUser),
            ChildEnd::AlreadyGone => Ok(()),
        }
    }

    /// As `stop`, for a request that starts the agent again at once. A child
    /// that ended on its own just before is recorded as stopped here, because
    /// its exit event waits for this request to finish and the start would
    /// otherwise find the agent still running.
    ///
    /// # Errors
    /// As `stop`.
    pub fn stop_for_restart(&self, agent: &mut Agent) -> Result<(), ApplicationError> {
        self.stop_child(agent)?;
        self.record_end(agent, StatusEvent::StoppedByUser)
    }

    /// As `stop`, for the office putting an idle agent to sleep: the status
    /// becomes `asleep` and the session closes as it does for a stop.
    ///
    /// # Errors
    /// As `stop`.
    pub fn sleep(&self, agent: &mut Agent) -> Result<(), ApplicationError> {
        match self.stop_child(agent)? {
            ChildEnd::Stopped => self.record_end(agent, StatusEvent::PutToSleep),
            ChildEnd::AlreadyGone => Ok(()),
        }
    }

    fn stop_child(&self, agent: &Agent) -> Result<ChildEnd, ApplicationError> {
        let grace = Duration::from_millis(self.settings.load()?.stop_grace_ms);
        match self.terminals.stop(agent.id, grace) {
            Ok(()) => Ok(ChildEnd::Stopped),
            Err(TerminalHostError::NotRunning) => Ok(ChildEnd::AlreadyGone),
            Err(error) => Err(ApplicationError::Internal(error.to_string())),
        }
    }

    fn record_end(&self, agent: &mut Agent, event: StatusEvent) -> Result<(), ApplicationError> {
        agent.apply(event)?;
        self.agents.update_status(agent.id, agent.status)?;
        self.sessions
            .end_running(agent.id, &ExitResult::Signal, self.clock.now())?;
        Ok(())
    }
}
