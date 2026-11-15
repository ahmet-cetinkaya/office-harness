// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::entities::{Agent, Project};
use agent_domain::value_objects::ProjectId;
use kernel::AgentId;

use crate::common::errors::ApplicationError;
use crate::common::interfaces::{AgentStore, ProjectStore};

/// # Errors
/// `NotFound` naming the agent id, or the store's error.
pub fn require_agent(store: &dyn AgentStore, id: AgentId) -> Result<Agent, ApplicationError> {
    store
        .find(id)?
        .ok_or_else(|| ApplicationError::NotFound(format!("agent {id} was not found")))
}

/// # Errors
/// `NotFound` naming the project id, or the store's error.
pub fn require_project(
    store: &dyn ProjectStore,
    id: ProjectId,
) -> Result<Project, ApplicationError> {
    store
        .find(id)?
        .ok_or_else(|| ApplicationError::NotFound(format!("project {id} was not found")))
}
