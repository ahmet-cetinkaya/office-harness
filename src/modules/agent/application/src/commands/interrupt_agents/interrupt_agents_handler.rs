// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::time::Duration;

use agent_domain::entities::Agent;
use agent_domain::enums::StatusEvent;
use agent_domain::value_objects::ExitResult;

use crate::common::errors::{ApplicationError, TerminalHostError};
use crate::common::interfaces::{AgentStore, Clock, SessionStore, SettingsStore, TerminalHost};

/// What stopping the server did to the running agents.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct InterruptOutcome {
    /// The agents now `interrupted`, with the status they show.
    pub interrupted: Vec<Agent>,
    /// One text for each agent whose child could not be ended.
    pub failures: Vec<String>,
}

/// Ends every running agent when the server stops, on a signal or on
/// `server.restart`: the child is stopped, the agent is `interrupted`, and
/// its session ends as `interrupted` (FR-004). Only an explicit start moves
/// an agent out of `interrupted`.
#[derive(Clone)]
pub struct InterruptAgentsHandler {
    agents: Arc<dyn AgentStore>,
    settings: Arc<dyn SettingsStore>,
    sessions: Arc<dyn SessionStore>,
    terminals: Arc<dyn TerminalHost>,
    clock: Arc<dyn Clock>,
}

impl InterruptAgentsHandler {
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

    /// # Errors
    /// `Storage` when the agents cannot be listed. A failure for one agent
    /// does not stop the others; it is in the outcome.
    pub fn handle(&self) -> Result<InterruptOutcome, ApplicationError> {
        let grace = Duration::from_millis(self.settings.load()?.stop_grace_ms);
        let mut outcome = InterruptOutcome::default();
        for mut agent in self.agents.list()? {
            if !agent.status.is_running() {
                continue;
            }
            match self.interrupt(&mut agent, grace) {
                Ok(()) => outcome.interrupted.push(agent),
                Err(error) => outcome
                    .failures
                    .push(format!("agent {}: {error}", agent.name)),
            }
        }
        Ok(outcome)
    }

    fn interrupt(&self, agent: &mut Agent, grace: Duration) -> Result<(), ApplicationError> {
        match self.terminals.stop(agent.id, grace) {
            // The child ended on its own a moment ago.
            Ok(()) | Err(TerminalHostError::NotRunning) => {}
            Err(error) => return Err(ApplicationError::Internal(error.to_string())),
        }
        agent.apply(StatusEvent::ServerStopped)?;
        self.agents.update_status(agent.id, agent.status)?;
        self.sessions
            .end_running(agent.id, &ExitResult::Interrupted, self.clock.now())?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "interrupt_agents_handler.tests.rs"]
mod tests;
