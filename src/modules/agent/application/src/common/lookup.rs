// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use agent_domain::entities::{Agent, Project};
use agent_domain::errors::AgentError;
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

/// The folder an agent's CLI starts in, with the name of the field it came
/// from: the working folder, else the project's folder (0001/FR-012,
/// FR-013). The folder is not checked on disk.
///
/// # Errors
/// `Invalid` for an agent with neither, `NotFound` for its missing project.
pub fn start_folder(
    store: &dyn ProjectStore,
    agent: &Agent,
) -> Result<(&'static str, PathBuf), ApplicationError> {
    match (&agent.working_folder, agent.project_id) {
        (Some(folder), _) => Ok(("working folder", folder.clone())),
        (None, Some(project_id)) => {
            Ok(("project folder", require_project(store, project_id)?.folder))
        }
        (None, None) => Err(AgentError::NoWorkingFolderOrProject.into()),
    }
}
