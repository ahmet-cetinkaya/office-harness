// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;
use std::sync::Arc;

use agent_domain::errors::AgentError;
use agent_domain::value_objects::{ProjectId, Settings};

use crate::common::errors::ApplicationError;
use crate::common::interfaces::{
    DEPENDENCY_LINKS_SUPPORTED, ProjectStore, SettingsStore, WorktreeInfo, WorktreeManager,
};
use crate::common::lookup::require_project;

/// Whether a folder is inside a git repository, with the base and the
/// branch name a create would use (`agent.worktree_info`, 0010 FR-001).
#[derive(Clone)]
pub struct WorktreeInfoHandler {
    projects: Arc<dyn ProjectStore>,
    worktrees: Option<Arc<dyn WorktreeManager>>,
    /// Absent takes the default settings (0021 FR-008).
    settings: Option<Arc<dyn SettingsStore>>,
}

impl WorktreeInfoHandler {
    /// Without a worktree service every folder reads as outside a
    /// repository, so the choice is never offered.
    #[must_use]
    pub fn new(
        projects: Arc<dyn ProjectStore>,
        worktrees: Option<Arc<dyn WorktreeManager>>,
    ) -> Self {
        Self {
            projects,
            worktrees,
            settings: None,
        }
    }

    /// Attaches the office settings, so the answer says whether a create
    /// would share the installed packages (0021 FR-008).
    #[must_use]
    pub fn with_settings(mut self, settings: Arc<dyn SettingsStore>) -> Self {
        self.settings = Some(settings);
        self
    }

    /// `None` when the folder is not inside a git repository.
    ///
    /// # Errors
    /// `Invalid` for no folder and no project, or a relative folder;
    /// `NotFound` for an unknown project; `Storage` when the settings
    /// cannot be read.
    pub fn handle(
        &self,
        folder: Option<PathBuf>,
        project_id: Option<ProjectId>,
        agent_name: &str,
    ) -> Result<Option<WorktreeInfo>, ApplicationError> {
        let folder = folder.filter(|folder| !folder.as_os_str().is_empty());
        let chosen = match (folder, project_id) {
            (Some(folder), _) => folder,
            (None, Some(project_id)) => require_project(&*self.projects, project_id)?.folder,
            (None, None) => {
                return Err(ApplicationError::Invalid(
                    "a folder or a project is needed".to_owned(),
                ));
            }
        };
        if !chosen.is_absolute() {
            return Err(AgentError::RelativePath {
                field: "working folder",
            }
            .into());
        }
        let Some(worktrees) = &self.worktrees else {
            return Ok(None);
        };
        let Some(mut info) = worktrees.info(&chosen, agent_name)? else {
            return Ok(None);
        };
        info.shares_dependencies = DEPENDENCY_LINKS_SUPPORTED && self.links_dependencies()?;
        Ok(Some(info))
    }

    fn links_dependencies(&self) -> Result<bool, ApplicationError> {
        let settings = match &self.settings {
            Some(store) => store.load()?,
            None => Settings::default(),
        };
        Ok(settings.worktree_dependency_links)
    }
}
