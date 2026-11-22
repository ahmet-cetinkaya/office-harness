// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;
use std::sync::Arc;

use agent_domain::services::{MAX_SKIP, RelativePath};
use agent_domain::value_objects::is_commit_id;
use kernel::AgentId;

use crate::common::errors::ApplicationError;
use crate::common::interfaces::{AgentStore, DiffRequest, FileView, ProjectStore};
use crate::common::lookup::{require_agent, start_folder};
use crate::common::models::{
    ChangeList, CommitDetail, CommitPage, FileContent, FileDiff, FolderListing,
};

const COMMIT_FORMAT: &str = "commit must be 40 or 64 lowercase hexadecimal characters";

/// The paths, commit, and reveal choice of one `agent.files.diff`, as the
/// request carries them.
#[derive(Debug, Clone, Copy)]
pub struct DiffQuestion<'a> {
    pub path: &'a str,
    pub old_path: Option<&'a str>,
    pub commit: Option<&'a str>,
    pub reveal: bool,
}

/// The read-only file view of one agent (`agent.files.*`, 0016): finds the
/// agent's start folder again on every call and asks the `FileView` port.
#[derive(Clone)]
pub struct FileViewHandler {
    agents: Arc<dyn AgentStore>,
    projects: Arc<dyn ProjectStore>,
    view: Arc<dyn FileView>,
}

impl FileViewHandler {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        projects: Arc<dyn ProjectStore>,
        view: Arc<dyn FileView>,
    ) -> Self {
        Self {
            agents,
            projects,
            view,
        }
    }

    /// # Errors
    /// `NotFound` for an unknown agent; `Invalid`, `Refused`, `Internal`
    /// from the view.
    pub fn list(&self, agent_id: AgentId, path: &str) -> Result<FolderListing, ApplicationError> {
        let path = RelativePath::parse_folder("path", path)?;
        Ok(self.view.list(&self.root_of(agent_id)?, &path)?)
    }

    /// # Errors
    /// As for `list`.
    pub fn read(
        &self,
        agent_id: AgentId,
        path: &str,
        reveal: bool,
    ) -> Result<FileContent, ApplicationError> {
        let path = RelativePath::parse("path", path)?;
        Ok(self.view.read(&self.root_of(agent_id)?, &path, reveal)?)
    }

    /// # Errors
    /// `NotFound` for an unknown agent; `Internal` from the view.
    pub fn changes(&self, agent_id: AgentId) -> Result<ChangeList, ApplicationError> {
        Ok(self.view.changes(&self.root_of(agent_id)?)?)
    }

    /// # Errors
    /// As for `list`; `Invalid` for a commit that is not a full commit id.
    pub fn diff(
        &self,
        agent_id: AgentId,
        question: &DiffQuestion<'_>,
    ) -> Result<FileDiff, ApplicationError> {
        let request = DiffRequest {
            path: RelativePath::parse("path", question.path)?,
            old_path: question
                .old_path
                .map(|old_path| RelativePath::parse("old_path", old_path))
                .transpose()?,
            commit: question.commit.map(full_commit_id).transpose()?,
            reveal: question.reveal,
        };
        Ok(self.view.diff(&self.root_of(agent_id)?, &request)?)
    }

    /// # Errors
    /// `NotFound` for an unknown agent; `Invalid` for a `skip` over
    /// 1,000,000; `Internal` from the view.
    pub fn history(&self, agent_id: AgentId, skip: u32) -> Result<CommitPage, ApplicationError> {
        if skip > MAX_SKIP {
            return Err(ApplicationError::Invalid(format!(
                "skip must be at most {MAX_SKIP}"
            )));
        }
        Ok(self.view.history(&self.root_of(agent_id)?, skip)?)
    }

    /// # Errors
    /// `NotFound` for an unknown agent; `Invalid` for a commit that is not a
    /// full commit id or not one of the repository; `Internal` from the
    /// view.
    pub fn commit(
        &self,
        agent_id: AgentId,
        commit: &str,
    ) -> Result<CommitDetail, ApplicationError> {
        let commit = full_commit_id(commit)?;
        Ok(self.view.commit(&self.root_of(agent_id)?, &commit)?)
    }

    fn root_of(&self, agent_id: AgentId) -> Result<PathBuf, ApplicationError> {
        let agent = require_agent(&*self.agents, agent_id)?;
        let (_, folder) = start_folder(&*self.projects, &agent)?;
        Ok(folder)
    }
}

/// A commit reaches git only in this form, so it can never read as an
/// option or a revision expression.
fn full_commit_id(text: &str) -> Result<String, ApplicationError> {
    if is_commit_id(text) {
        return Ok(text.to_owned());
    }
    Err(ApplicationError::Invalid(COMMIT_FORMAT.to_owned()))
}

#[cfg(test)]
#[path = "file_view_handler.tests.rs"]
mod tests;
