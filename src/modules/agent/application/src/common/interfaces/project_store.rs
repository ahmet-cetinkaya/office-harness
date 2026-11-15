// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::entities::Project;
use agent_domain::value_objects::ProjectId;

use crate::common::errors::PortError;

/// The stored projects.
pub trait ProjectStore: Send + Sync {
    /// # Errors
    /// `Duplicate` when another project has the name, ignoring case.
    fn insert(&self, project: &Project) -> Result<(), PortError>;

    /// # Errors
    /// `Storage`.
    fn find(&self, id: ProjectId) -> Result<Option<Project>, PortError>;

    /// # Errors
    /// `Storage`.
    fn list(&self) -> Result<Vec<Project>, PortError>;

    /// # Errors
    /// `ProjectInUse`, naming the agents, when agents still use the project,
    /// and `NotFound` when it does not exist.
    fn delete(&self, id: ProjectId) -> Result<(), PortError>;
}
