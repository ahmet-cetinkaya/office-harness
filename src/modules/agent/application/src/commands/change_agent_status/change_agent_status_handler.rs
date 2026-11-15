// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_domain::entities::Agent;
use agent_domain::enums::StatusEvent;
use kernel::AgentId;

use crate::common::errors::ApplicationError;
use crate::common::interfaces::AgentStore;

/// Moves an agent along the status state machine when a terminal event
/// arrives (FR-014). The bridge turns terminal events into `StatusEvent`.
#[derive(Clone)]
pub struct ChangeAgentStatusHandler {
    agents: Arc<dyn AgentStore>,
}

impl ChangeAgentStatusHandler {
    #[must_use]
    pub fn new(agents: Arc<dyn AgentStore>) -> Self {
        Self { agents }
    }

    /// Returns the agent with its new status, or `None` when nothing changed:
    /// the agent is gone (it was deleted while its child exited), or the
    /// event does not apply in its status (a stop already set `stopped`).
    ///
    /// # Errors
    /// `Storage`.
    pub fn handle(
        &self,
        agent_id: AgentId,
        event: StatusEvent,
    ) -> Result<Option<Agent>, ApplicationError> {
        let Some(mut agent) = self.agents.find(agent_id)? else {
            return Ok(None);
        };
        if agent.apply(event).is_err() {
            return Ok(None);
        }
        self.agents.update_status(agent_id, agent.status)?;
        Ok(Some(agent))
    }
}

#[cfg(test)]
#[path = "change_agent_status_handler.tests.rs"]
mod tests;
