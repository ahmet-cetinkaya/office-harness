// SPDX-License-Identifier: AGPL-3.0-or-later

//! `FileView` over the local disk and the system git program (0016): the
//! folder reader and the git reader behind one port.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_application::common::interfaces::{DiffRequest, FileView, FileViewError};
use agent_application::common::models::{
    ChangeList, CommitDetail, CommitPage, DiffState, FileContent, FileDiff, FolderListing,
};
use agent_domain::services::RelativePath;

use crate::services::folder_reader::{
    canonical_root, contained_path, has_secret_name, list_folder, read_file,
};
use crate::services::git_reader::GitReader;
use crate::services::git_runner::LazyGitRunner;

/// Reads the folder, its changes, and its history without writing.
pub struct LocalFileView {
    git: GitReader,
}

impl LocalFileView {
    /// Git is found on first use, through the runner the worktree service
    /// shares.
    #[must_use]
    pub fn new(runner: Arc<LazyGitRunner>) -> Self {
        Self {
            git: GitReader::new(runner),
        }
    }
}

impl FileView for LocalFileView {
    fn list(&self, root: &Path, path: &RelativePath) -> Result<FolderListing, FileViewError> {
        list_folder(root, path)
    }

    fn read(
        &self,
        root: &Path,
        path: &RelativePath,
        reveal: bool,
    ) -> Result<FileContent, FileViewError> {
        read_file(root, path, reveal)
    }

    fn changes(&self, root: &Path) -> Result<ChangeList, FileViewError> {
        self.git.changes(&canonical_root(root)?)
    }

    fn diff(&self, root: &Path, request: &DiffRequest) -> Result<FileDiff, FileViewError> {
        let root = canonical_root(root)?;
        // Containment is checked for a revealed diff too.
        if has_secret_side(&root, request)? && !request.reveal {
            return Ok(FileDiff {
                path: request.path.to_text(),
                state: DiffState::Secret,
                size: None,
                bytes: None,
            });
        }
        self.git.diff(&root, request)
    }

    fn history(&self, root: &Path, skip: u32) -> Result<CommitPage, FileViewError> {
        self.git.history(&canonical_root(root)?, skip)
    }

    fn commit(&self, root: &Path, commit: &str) -> Result<CommitDetail, FileViewError> {
        self.git.commit(&canonical_root(root)?, commit)
    }
}

/// True when the path or the old path of a rename is secret-looking, by
/// name or, for a path on the disk, by the name it resolves to. Both go
/// through the same containment, so a rename cannot name a file outside the
/// root (review r1, C1). A path of a commit names a file of git's tree, not
/// of the disk.
fn has_secret_side(root: &Path, request: &DiffRequest) -> Result<bool, FileViewError> {
    for path in std::iter::once(&request.path).chain(request.old_path.as_ref()) {
        let resolved = if request.commit.is_some() {
            PathBuf::from(path.to_text())
        } else {
            contained_path(root, path)?
        };
        if has_secret_name(path, &resolved) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
#[path = "local_file_view.tests.rs"]
mod tests;
