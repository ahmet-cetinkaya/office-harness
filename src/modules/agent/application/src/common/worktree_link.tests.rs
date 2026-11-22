// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs;

use tempfile::TempDir;

use super::*;

/// `git_folder/worktrees/<name>`, the record of one linked worktree as git
/// lays it out, with its `commondir` file.
fn worktree_record(git_folder: &Path, name: &str) -> PathBuf {
    let record = git_folder.join("worktrees").join(name);
    fs::create_dir_all(&record).expect("record");
    fs::write(record.join(COMMONDIR_FILE), "../..\n").expect("commondir");
    record
}

fn link(folder: &Path, target: &Path) {
    fs::create_dir_all(folder).expect("worktree folder");
    fs::write(
        folder.join(DOT_GIT),
        format!("{GITDIR_PREFIX} {}\n", target.display()),
    )
    .expect("link");
}

#[test]
fn a_worktree_of_the_repository_is_linked() {
    let temp = TempDir::new().expect("temp");
    let repository = temp.path().join("repo");
    let worktree = temp.path().join("wt");
    link(&worktree, &worktree_record(&repository.join(DOT_GIT), "wt"));

    assert!(is_linked_worktree(&worktree, &repository));
}

#[test]
fn a_link_turned_to_another_repository_is_refused() {
    let temp = TempDir::new().expect("temp");
    let repository = temp.path().join("repo");
    fs::create_dir_all(repository.join(DOT_GIT)).expect("repository");
    let worktree = temp.path().join("wt");
    let other = temp.path().join("other");
    link(&worktree, &worktree_record(&other.join(DOT_GIT), "wt"));

    assert!(!is_linked_worktree(&worktree, &repository));
}

#[test]
fn a_missing_git_folder_is_refused() {
    let temp = TempDir::new().expect("temp");
    let repository = temp.path().join("repo");
    fs::create_dir_all(repository.join(DOT_GIT)).expect("repository");
    let worktree = temp.path().join("wt");
    link(&worktree, &temp.path().join("moved"));

    assert!(!is_linked_worktree(&worktree, &repository));
}

#[test]
fn a_repository_with_a_separate_git_folder_and_a_relative_link_is_linked() {
    let temp = TempDir::new().expect("temp");
    let repository = temp.path().join("repo");
    let git_folder = temp.path().join("data.git");
    link(&repository, &git_folder);
    let worktree = temp.path().join("wt");
    worktree_record(&git_folder, "wt");
    link(&worktree, Path::new("../data.git/worktrees/wt"));

    assert!(is_linked_worktree(&worktree, &repository));
}
