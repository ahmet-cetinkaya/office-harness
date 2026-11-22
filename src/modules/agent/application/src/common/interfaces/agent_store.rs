// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::entities::Agent;
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::TaskText;
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

    /// Stores the task text, or clears it for `None`.
    ///
    /// # Errors
    /// `NotFound` when the agent does not exist.
    fn update_task(&self, id: AgentId, task: Option<&TaskText>) -> Result<(), PortError>;

    /// # Errors
    /// `NotFound` when the agent does not exist.
    fn delete(&self, id: AgentId) -> Result<(), PortError>;
}
