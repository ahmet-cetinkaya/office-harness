// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use audit_domain::ActionEntry;
use kernel::AgentId;

use crate::common::errors::AuditError;
use crate::common::interfaces::ActionLog;

/// The action record entries of one agent (`agent.actions`).
#[derive(Clone)]
pub struct ListActionsHandler {
    log: Arc<dyn ActionLog>,
}

impl ListActionsHandler {
    #[must_use]
    pub fn new(log: Arc<dyn ActionLog>) -> Self {
        Self { log }
    }

    /// # Errors
    /// The log's error.
    pub fn handle(&self, agent_id: AgentId) -> Result<Vec<ActionEntry>, AuditError> {
        self.log.list_for_agent(agent_id)
    }
}

#[cfg(test)]
#[path = "list_actions_handler.tests.rs"]
mod tests;
