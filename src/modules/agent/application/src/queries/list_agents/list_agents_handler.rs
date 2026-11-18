// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_domain::entities::Agent;
use kernel::AgentId;

use crate::common::errors::ApplicationError;
use crate::common::interfaces::AgentStore;

/// Every agent with its status (`agent.list`).
#[derive(Clone)]
pub struct ListAgentsHandler {
    agents: Arc<dyn AgentStore>,
}

impl ListAgentsHandler {
    #[must_use]
    pub fn new(agents: Arc<dyn AgentStore>) -> Self {
        Self { agents }
    }

    /// # Errors
    /// `Storage`.
    pub fn handle(&self) -> Result<Vec<Agent>, ApplicationError> {
        Ok(self.agents.list()?)
    }

    /// One agent, read back after another use case created it (the role
    /// create of 0002). `None` becomes `NotFound`, because the caller named
    /// the agent it just made.
    ///
    /// # Errors
    /// `NotFound` when no agent carries the id; `Storage` when the read
    /// fails.
    pub fn handle_one(&self, agent_id: AgentId) -> Result<Agent, ApplicationError> {
        self.agents
            .find(agent_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("agent {agent_id} does not exist")))
    }
}
