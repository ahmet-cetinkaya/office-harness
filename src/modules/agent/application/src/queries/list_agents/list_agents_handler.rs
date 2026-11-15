// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_domain::entities::Agent;

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
}
