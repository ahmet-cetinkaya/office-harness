// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;

use agent_domain::services::RelativePath;
use thiserror::Error;

use crate::common::errors::ApplicationError;
use crate::common::models::{
    ChangeList, CommitDetail, CommitPage, FileContent, FileDiff, FolderListing,
};

/// Why a file view read failed. The text is the user-facing reason.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum FileViewError {
    #[error("{0}")]
    Invalid(String),
    /// The path leaves the root.
    #[error("{0}")]
    Refused(String),
    #[error("{0}")]
    Failed(String),
}

impl From<FileViewError> for ApplicationError {
    fn from(error: FileViewError) -> Self {
        match error {
            FileViewError::Invalid(message) => Self::Invalid(message),
            FileViewError::Refused(message) => Self::Refused(message),
            FileViewError::Failed(message) => Self::Internal(message),
        }
    }
}

/// One diff question: the paths are relative to the root, the commit is a
/// full commit id (0016 contract `agent.files.diff`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRequest {
    pub path: RelativePath,
    pub old_path: Option<RelativePath>,
    pub commit: Option<String>,
    pub reveal: bool,
}

/// The read-only view of one folder, its changes against HEAD, and its
/// commit history. No method writes anything. `root` is the agent's start
/// folder, not yet resolved; every method resolves it again.
pub trait FileView: Send + Sync {
    /// # Errors
    /// `Invalid` for a file, `Refused` outside the root, `Failed` when the
    /// folder cannot be read.
    fn list(&self, root: &Path, path: &RelativePath) -> Result<FolderListing, FileViewError>;

    /// # Errors
    /// `Invalid` for a folder or an entry that is not a regular file,
    /// `Refused` outside the root, `Failed` when the file cannot be read.
    fn read(
        &self,
        root: &Path,
        path: &RelativePath,
        reveal: bool,
    ) -> Result<FileContent, FileViewError>;

    /// # Errors
    /// `Failed` when git is not available or does not answer.
    fn changes(&self, root: &Path) -> Result<ChangeList, FileViewError>;

    /// # Errors
    /// `Invalid` for a commit that is not one of the repository, `Refused`
    /// outside the root, `Failed` when git does not answer.
    fn diff(&self, root: &Path, request: &DiffRequest) -> Result<FileDiff, FileViewError>;

    /// # Errors
    /// `Failed` when git is not available or does not answer.
    fn history(&self, root: &Path, skip: u32) -> Result<CommitPage, FileViewError>;

    /// # Errors
    /// `Invalid` for a commit that is not one of the repository, `Failed`
    /// when git does not answer.
    fn commit(&self, root: &Path, commit: &str) -> Result<CommitDetail, FileViewError>;
}
