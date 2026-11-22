// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::{Path, PathBuf};

const DOT_GIT: &str = ".git";
const GITDIR_PREFIX: &str = "gitdir:";
const COMMONDIR_FILE: &str = "commondir";

/// True when the worktree's `.git` file names a git folder that exists and
/// shares its git data with `repository`: the repository has not moved or
/// gone, and the link was not turned to another repository (0010 FR-008).
#[must_use]
pub fn is_linked_worktree(folder: &Path, repository: &Path) -> bool {
    let worktree_data = linked_git_folder(folder).and_then(|git| common_git_folder(&git));
    let repository_data = repository_git_folder(repository).and_then(|git| common_git_folder(&git));
    worktree_data.is_some() && worktree_data == repository_data
}

/// The folder a `.git` file names; a relative target is read from `folder`.
fn linked_git_folder(folder: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(folder.join(DOT_GIT)).ok()?;
    let target = text.trim().strip_prefix(GITDIR_PREFIX)?.trim();
    let git_folder = folder.join(target);
    git_folder.is_dir().then_some(git_folder)
}

/// The repository's git folder: its `.git` folder, or the folder its `.git`
/// file names (a separate git folder, or a bare repository's worktree).
fn repository_git_folder(repository: &Path) -> Option<PathBuf> {
    let dot_git = repository.join(DOT_GIT);
    if dot_git.is_dir() {
        return Some(dot_git);
    }
    linked_git_folder(repository)
}

/// The canonical folder that holds the shared git data: the one the git
/// folder's `commondir` file names, else the git folder itself.
fn common_git_folder(git_folder: &Path) -> Option<PathBuf> {
    let common = match std::fs::read_to_string(git_folder.join(COMMONDIR_FILE)) {
        Ok(text) => git_folder.join(text.trim_end_matches(['\n', '\r'])),
        Err(_) => git_folder.to_path_buf(),
    };
    common.canonicalize().ok()
}

#[cfg(test)]
#[path = "worktree_link.tests.rs"]
mod tests;
