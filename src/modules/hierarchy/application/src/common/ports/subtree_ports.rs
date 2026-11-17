// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;

use crate::common::errors::HierarchyApplicationError;

/// What can go wrong when the agent module stops or deletes one agent on the
/// hierarchy's behalf. The text is the agent module's own message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtreeActionError(pub String);

/// Stops one agent, as `agent.stop` does (FR-007).
pub trait SubtreeAgentEnder: Send + Sync {
    /// # Errors
    /// The agent module's failure when the agent cannot be stopped.
    fn stop(&self, agent_id: AgentId) -> Result<(), SubtreeActionError>;
}

/// Deletes one agent, as `agent.delete` does (FR-006).
pub trait SubtreeAgentRemover: Send + Sync {
    /// # Errors
    /// The agent module's failure when the agent cannot be deleted.
    fn delete(&self, agent_id: AgentId) -> Result<(), SubtreeActionError>;
}

impl SubtreeActionError {
    /// Maps the failure to the application error of a subtree action.
    #[must_use]
    pub fn into_application(self) -> HierarchyApplicationError {
        HierarchyApplicationError::Conflict(self.0)
    }
}
