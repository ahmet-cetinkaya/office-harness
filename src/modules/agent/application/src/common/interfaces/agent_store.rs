// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::entities::Agent;
use agent_domain::enums::AgentStatus;
use kernel::AgentId;

use crate::common::errors::PortError;

/// The stored agents.
pub trait AgentStore: Send + Sync {
    /// # Errors
    /// `Duplicate` when another agent has the name, ignoring case.
    fn insert(&self, agent: &Agent) -> Result<(), PortError>;

    /// # Errors
    /// `Storage`.
    fn find(&self, id: AgentId) -> Result<Option<Agent>, PortError>;

    /// # Errors
    /// `Storage`.
    fn list(&self) -> Result<Vec<Agent>, PortError>;

    /// # Errors
    /// `NotFound` when the agent does not exist.
    fn update_status(&self, id: AgentId, status: AgentStatus) -> Result<(), PortError>;

    /// # Errors
    /// `NotFound` when the agent does not exist.
    fn delete(&self, id: AgentId) -> Result<(), PortError>;
}
