// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::{Path, PathBuf};

use agent_domain::value_objects::AgentWorktree;
use kernel::AgentId;
use thiserror::Error;

use crate::common::errors::ApplicationError;

/// What the create form shows for a folder inside a git repository (0010
/// FR-001, D4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeInfo {
    /// The repository's top folder.
    pub top: PathBuf,
    /// The chosen folder inside the repository, `/`-separated, empty at the
    /// top.
    pub subfolder: String,
    /// The checked-out branch, or the full commit id when detached.
    pub base: String,
    pub base_is_branch: bool,
    pub branches: Vec<String>,
    /// The branch a create would make now.
    pub branch_name: String,
}

/// One own-worktree create (0010 D5). Absent `base` and `branch_name` take
/// the folder's own base and the free name of now.
#[derive(Debug, Clone, Copy)]
pub struct WorktreeCreate<'a> {
    pub folder: &'a Path,
    pub agent_id: AgentId,
    pub agent_name: &'a str,
    pub base: Option<&'a str>,
    pub branch_name: Option<&'a str>,
}

/// A made worktree and the folder the agent starts in: the worktree plus
/// the chosen folder's subfolder (FR-002).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedWorktree {
    pub worktree: AgentWorktree,
    pub working_folder: PathBuf,
}

/// The state of one worktree for the delete question (0010 D6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeStatus {
    pub folder_exists: bool,
    /// False when the repository does not answer or the status cannot be
    /// read: such a worktree is always kept.
    pub reachable: bool,
    /// Changed, added, deleted, and untracked files; ignored files are not
    /// counted (A9).
    pub changed_files: usize,
    /// A hash of the status, valid within one server run.
    pub status_hash: String,
    /// Commits of the branch that the base lacks; `None` when unknown.
    pub own_commits: Option<usize>,
    pub base_exists: bool,
}

impl WorktreeStatus {
    /// The branch holds nothing the base lacks, so deleting it loses no
    /// commit (FR-007).
    #[must_use]
    pub fn branch_is_deletable(&self) -> bool {
        self.base_exists && self.own_commits == Some(0)
    }
}

/// Why a worktree action failed. The text is the user-facing reason.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum WorktreeError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Failed(String),
}

impl From<WorktreeError> for ApplicationError {
    fn from(error: WorktreeError) -> Self {
        match error {
            WorktreeError::Invalid(message) => Self::Invalid(message),
            WorktreeError::Conflict(message) => Self::Conflict(message),
            WorktreeError::Failed(message) => Self::Internal(message),
        }
    }
}

/// The repository's git worktrees, as the office makes and removes them
/// (0010). No method writes git configuration or runs a hook.
pub trait WorktreeManager: Send + Sync {
    /// `None` when the folder is not inside a git repository or no usable
    /// git is installed.
    ///
    /// # Errors
    /// `Invalid` when the branch name cannot be made valid.
    fn info(&self, folder: &Path, agent_name: &str) -> Result<Option<WorktreeInfo>, WorktreeError>;

    /// Makes the branch and the worktree. A failure leaves neither.
    ///
    /// # Errors
    /// `Invalid` for a folder outside a repository, a base or branch name
    /// the info does not allow, or a subfolder missing in the base;
    /// `Conflict` for a taken branch name; `Failed` with git's reason.
    fn create(&self, request: &WorktreeCreate<'_>) -> Result<CreatedWorktree, WorktreeError>;

    /// Reads the state again; never fails, an unreadable worktree reads as
    /// not reachable.
    fn status(&self, worktree: &AgentWorktree) -> WorktreeStatus;

    /// Removes the worktree folder and its record. Without `force`, git
    /// refuses a worktree with changes.
    ///
    /// # Errors
    /// `Invalid` for a folder outside the office's worktrees folder;
    /// `Failed` with git's reason.
    fn remove(&self, worktree: &AgentWorktree, force: bool) -> Result<(), WorktreeError>;

    /// # Errors
    /// `Failed` with git's reason, for example a branch checked out
    /// elsewhere.
    fn delete_branch(&self, worktree: &AgentWorktree) -> Result<(), WorktreeError>;

    /// The undo of a create: removes the worktree, its folder, and its
    /// branch without asking. Best effort; nothing to report.
    fn discard(&self, worktree: &AgentWorktree);
}
