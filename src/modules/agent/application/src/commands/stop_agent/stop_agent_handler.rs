// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_domain::entities::Agent;
use kernel::AgentId;

use crate::common::action_journal::ActionJournal;
use crate::common::errors::ApplicationError;
use crate::common::interfaces::AgentStore;
use crate::common::lookup::require_agent;
use crate::common::models::AgentAction;
use crate::common::session_ender::SessionEnder;

/// Stops a running agent (`agent.stop`, FR-012): the terminal asks, waits the
/// grace period, then forces.
#[derive(Clone)]
pub struct StopAgentHandler {
    agents: Arc<dyn AgentStore>,
    ender: SessionEnder,
    journal: ActionJournal,
}

impl StopAgentHandler {
    #[must_use]
    pub fn new(agents: Arc<dyn AgentStore>, ender: SessionEnder, journal: ActionJournal) -> Self {
        Self {
            agents,
            ender,
            journal,
        }
    }

    /// # Errors
    /// `NotFound`; `Conflict` when the agent does not run; `Internal` when the
    /// child cannot be ended, which leaves the status as it was.
    pub fn handle(&self, agent_id: AgentId) -> Result<Agent, ApplicationError> {
        let mut agent = require_agent(&*self.agents, agent_id)?;
        let outcome = self.end(&mut agent);
        let failure = outcome.as_ref().err().map(ToString::to_string);
        self.journal
            .record(agent_id, AgentAction::Stop, failure.as_deref())?;
        outcome?;
        // The stored status is the truth: a stop that found no child to end
        // changed nothing, so the copy read above may be stale.
        require_agent(&*self.agents, agent_id)
    }

    fn end(&self, agent: &mut Agent) -> Result<(), ApplicationError> {
        if !agent.status.is_running() {
            return Err(ApplicationError::Conflict(format!(
                "agent {} is not running, it is {}",
                agent.name, agent.status
            )));
        }
        self.ender.stop(agent)
    }
}

#[cfg(test)]
#[path = "stop_agent_handler.tests.rs"]
mod tests;
