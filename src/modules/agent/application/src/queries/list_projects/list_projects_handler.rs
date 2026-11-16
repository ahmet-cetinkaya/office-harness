// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_domain::entities::Project;

use crate::common::errors::ApplicationError;
use crate::common::interfaces::ProjectStore;

/// Every project (`project.list`).
#[derive(Clone)]
pub struct ListProjectsHandler {
    projects: Arc<dyn ProjectStore>,
}

impl ListProjectsHandler {
    #[must_use]
    pub fn new(projects: Arc<dyn ProjectStore>) -> Self {
        Self { projects }
    }

    /// # Errors
    /// `Storage`.
    pub fn handle(&self) -> Result<Vec<Project>, ApplicationError> {
        Ok(self.projects.list()?)
    }
}
