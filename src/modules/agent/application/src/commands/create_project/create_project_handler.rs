// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;
use std::sync::Arc;

use agent_domain::entities::Project;
use agent_domain::value_objects::{DisplayName, ProjectId};

use crate::common::errors::ApplicationError;
use crate::common::interfaces::ProjectStore;

#[derive(Debug, Clone)]
pub struct CreateProjectRequest {
    pub name: String,
    pub folder: PathBuf,
}

/// Creates a project (`project.create`, FR-013).
#[derive(Clone)]
pub struct CreateProjectHandler {
    projects: Arc<dyn ProjectStore>,
}

impl CreateProjectHandler {
    #[must_use]
    pub fn new(projects: Arc<dyn ProjectStore>) -> Self {
        Self { projects }
    }

    /// # Errors
    /// `Invalid` for a blank or long name or a relative folder, and
    /// `Conflict` when another project has the name, ignoring case.
    pub fn handle(&self, request: CreateProjectRequest) -> Result<Project, ApplicationError> {
        let name = DisplayName::parse("project name", &request.name)?;
        let project = Project::new(ProjectId::generate(), name, request.folder)?;
        self.projects.insert(&project).map_err(|error| {
            ApplicationError::from_insert(error, "project name", project.name.as_str())
        })?;
        Ok(project)
    }
}

#[cfg(test)]
#[path = "create_project_handler.tests.rs"]
mod tests;
