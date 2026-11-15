// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_domain::value_objects::ProjectId;

use crate::common::errors::ApplicationError;
use crate::common::interfaces::ProjectStore;
use crate::common::lookup::require_project;

/// Deletes a project that no agent uses (`project.delete`, FR-013).
#[derive(Clone)]
pub struct DeleteProjectHandler {
    projects: Arc<dyn ProjectStore>,
}

impl DeleteProjectHandler {
    #[must_use]
    pub fn new(projects: Arc<dyn ProjectStore>) -> Self {
        Self { projects }
    }

    /// # Errors
    /// `NotFound`, or `Conflict` naming the agents that still use the project.
    pub fn handle(&self, project_id: ProjectId) -> Result<(), ApplicationError> {
        require_project(&*self.projects, project_id)?;
        Ok(self.projects.delete(project_id)?)
    }
}

#[cfg(test)]
#[path = "delete_project_handler.tests.rs"]
mod tests;
