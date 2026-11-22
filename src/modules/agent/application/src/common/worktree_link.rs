// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;

const GITDIR_PREFIX: &str = "gitdir:";

/// True when the worktree's `.git` file names a git folder that exists: the
/// repository has not moved or gone (0010 FR-008). A relative target is
/// read from the worktree folder.
#[must_use]
pub fn is_linked_worktree(folder: &Path) -> bool {
    std::fs::read_to_string(folder.join(".git"))
        .ok()
        .and_then(|text| {
            let target = text.trim().strip_prefix(GITDIR_PREFIX)?.trim().to_owned();
            Some(folder.join(target))
        })
        .is_some_and(|git_folder| git_folder.is_dir())
}
