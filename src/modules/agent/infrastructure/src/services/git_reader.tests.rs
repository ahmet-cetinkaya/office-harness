// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Write as _;
use std::fs;

use super::*;
use crate::services::file_view_fixture::Repo;

fn reader(repo: &Repo) -> GitReader {
    GitReader::new(repo.lazy_runner())
}

fn paths(files: &[ChangedFile]) -> Vec<(&str, ChangeKind)> {
    files
        .iter()
        .map(|file| (file.path.as_str(), file.kind))
        .collect()
}

fn working_request(path: &str) -> DiffRequest {
    DiffRequest {
        path: RelativePath::parse("path", path).expect("path"),
        old_path: None,
        commit: None,
        reveal: false,
    }
}

fn diff_text(diff: &FileDiff) -> String {
    String::from_utf8(diff.bytes.clone().expect("diff bytes")).expect("utf-8")
}

/// Commits `count` revisions of `folder/file.txt` for each `(folder, count)`
/// in one `fast-import`, so the commits form one line.
fn fast_commits(repo: &Repo, changes: &[(&str, usize)]) {
    let mut script = String::new();
    for (folder, index) in changes
        .iter()
        .flat_map(|(folder, count)| (0..*count).map(move |index| (*folder, index)))
    {
        let message = format!("change {folder} {index}");
        let content = format!("{index}\n");
        write!(
            script,
            "commit refs/heads/main\nauthor T <t@e.com> {time} +0000\ncommitter T <t@e.com> {time} +0000\ndata {}\n{message}\nM 644 inline {folder}/file.txt\ndata {}\n{content}\n",
            message.len(),
            content.len(),
            time = 1_700_000_000 + index,
        )
        .expect("a string takes text");
    }
    repo.git_input(&["fast-import", "--quiet", "--force"], script.as_bytes());
    repo.git(&["reset", "--quiet", "--hard", "main"]);
}

fn head_commit(repo: &Repo) -> String {
    repo.git(&["rev-parse", "HEAD"]).trim().to_owned()
}

#[test]
fn the_changes_of_the_root_list_every_kind_and_skip_ignored_files() {
    let repo = Repo::new();
    repo.write(".gitignore", "*.log\n");
    repo.write("modified.txt", "one\n");
    repo.write("deleted.txt", "gone\n");
    repo.write(
        "renamed-from.txt",
        "same content here\nmore lines\nand more\n",
    );
    repo.commit_all("first");
    repo.write("modified.txt", "two\n");
    fs::remove_file(repo.path("deleted.txt")).expect("remove");
    repo.git(&["mv", "renamed-from.txt", "renamed-to.txt"]);
    repo.write("added.txt", "new\n");
    repo.git(&["add", "added.txt"]);
    repo.write("untracked.txt", "u\n");
    repo.write("debug.log", "ignored\n");

    let changes = reader(&repo).changes(&repo.work).expect("changes");

    assert!(changes.repository);
    assert_eq!(
        paths(&changes.files),
        [
            ("added.txt", ChangeKind::Added),
            ("deleted.txt", ChangeKind::Deleted),
            ("modified.txt", ChangeKind::Modified),
            ("renamed-to.txt", ChangeKind::Renamed),
            ("untracked.txt", ChangeKind::Untracked),
        ]
    );
    assert_eq!(
        changes.files[3].old_path.as_deref(),
        Some("renamed-from.txt")
    );
    assert_eq!(changes.more, 0);
}

/// A file replaced by a link; links need Unix here.
#[cfg(unix)]
#[test]
fn a_file_replaced_by_a_link_is_a_type_change() {
    let repo = Repo::new();
    repo.write("target.txt", "file\n");
    repo.write("typed.txt", "file\n");
    repo.commit_all("first");
    fs::remove_file(repo.path("typed.txt")).expect("remove");
    std::os::unix::fs::symlink("target.txt", repo.path("typed.txt")).expect("link");

    let changes = reader(&repo).changes(&repo.work).expect("changes");

    assert_eq!(
        paths(&changes.files),
        [("typed.txt", ChangeKind::TypeChanged)]
    );
}

#[test]
fn a_subfolder_root_lists_its_own_paths_without_the_outside_ones() {
    let repo = Repo::new();
    repo.write("outside.txt", "outside\n");
    repo.write("sub/inside.txt", "inside\n");
    repo.write(
        "sub/moved.txt",
        "the moved content\nsecond line\nthird line\n",
    );
    repo.commit_all("first");
    repo.write("outside.txt", "changed outside\n");
    repo.write("sub/inside.txt", "changed inside\n");
    repo.git(&["mv", "sub/moved.txt", "moved-out.txt"]);
    repo.git(&["mv", "outside.txt", "sub/came-in.txt"]);

    let changes = reader(&repo)
        .changes(&repo.path("sub"))
        .expect("changes of the subfolder");

    // Git pairs a rename only inside the pathspec, so a path that came in
    // from outside is an addition and one that left is a deletion.
    assert_eq!(
        paths(&changes.files),
        [
            ("came-in.txt", ChangeKind::Added),
            ("inside.txt", ChangeKind::Modified),
            ("moved.txt", ChangeKind::Deleted),
        ]
    );
    assert!(changes.files.iter().all(|file| file.old_path.is_none()));
}

#[test]
fn more_than_a_thousand_changes_are_counted_in_more() {
    let repo = Repo::new();
    repo.commit_all("first");
    for index in 0..=MAX_CHANGED_FILES {
        repo.write(&format!("many/f{index:04}.txt"), "x");
    }

    let changes = reader(&repo).changes(&repo.work).expect("changes");

    assert_eq!(changes.files.len(), MAX_CHANGED_FILES);
    assert_eq!(changes.more, 1);
    assert_eq!(changes.files[0].path, "many/f0000.txt");
}

#[test]
fn a_folder_outside_any_repository_is_not_a_repository() {
    let temp = tempfile::TempDir::new().expect("temp folder");
    let repo = Repo::new();
    let root = temp.path().canonicalize().expect("canonical");

    let changes = reader(&repo).changes(&root).expect("changes");
    let history = reader(&repo).history(&root, 0).expect("history");

    assert!(!changes.repository && changes.files.is_empty());
    assert!(!history.repository && history.commits.is_empty());
    assert!(
        reader(&repo)
            .diff(&root, &working_request("a.txt"))
            .is_err()
    );
}

#[test]
fn without_git_every_git_answer_says_so() {
    let repo = Repo::new();
    let reader = GitReader::new(Arc::new(LazyGitRunner::fixed(None)));

    let error = reader.changes(&repo.work).expect_err("no git");

    assert_eq!(error, FileViewError::Failed(GIT_MISSING.to_owned()));
}

#[test]
fn a_repository_without_a_commit_lists_staged_and_untracked_files_and_diffs_them() {
    let repo = Repo::new();
    repo.write("staged.txt", "staged\n");
    repo.git(&["add", "staged.txt"]);
    repo.write("loose.txt", "loose\n");

    let changes = reader(&repo).changes(&repo.work).expect("changes");
    let staged = reader(&repo)
        .diff(&repo.work, &working_request("staged.txt"))
        .expect("diff against the empty tree");
    let history = reader(&repo).history(&repo.work, 0).expect("history");

    assert_eq!(
        paths(&changes.files),
        [
            ("loose.txt", ChangeKind::Untracked),
            ("staged.txt", ChangeKind::Added)
        ]
    );
    assert_eq!(diff_text(&staged), "@@ -0,0 +1 @@\n+staged\n");
    assert!(history.repository && history.commits.is_empty() && !history.more);
}

#[test]
fn the_diff_holds_staged_and_unstaged_changes_without_git_headers() {
    let repo = Repo::new();
    repo.write("a.txt", "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n");
    repo.commit_all("first");
    repo.write("a.txt", "1\nTWO\n3\n4\n5\n6\n7\n8\n9\n10\n");
    repo.git(&["add", "a.txt"]);
    repo.write("a.txt", "1\nTWO\n3\n4\n5\n6\n7\n8\n9\nTEN\n");

    let diff = reader(&repo)
        .diff(&repo.work, &working_request("a.txt"))
        .expect("diff");

    assert_eq!(diff.state, DiffState::Diff);
    let text = diff_text(&diff);
    assert!(text.starts_with("@@ "), "{text}");
    assert!(
        text.contains("-2\n+TWO\n") && text.contains("-10\n+TEN\n"),
        "{text}"
    );
    assert!(!text.contains("diff --git"));
}

#[test]
fn an_unchanged_path_has_no_diff_and_a_deleted_path_is_all_removed() {
    let repo = Repo::new();
    repo.write("same.txt", "same\n");
    repo.write("gone.txt", "bye\n");
    repo.commit_all("first");
    fs::remove_file(repo.path("gone.txt")).expect("remove");

    let same = reader(&repo)
        .diff(&repo.work, &working_request("same.txt"))
        .expect("diff");
    let gone = reader(&repo)
        .diff(&repo.work, &working_request("gone.txt"))
        .expect("diff");

    assert_eq!(same.state, DiffState::None);
    assert_eq!(diff_text(&gone), "@@ -1 +0,0 @@\n-bye\n");
}

#[test]
fn a_staged_rename_diffs_through_both_paths() {
    let repo = Repo::new();
    repo.write("old.txt", "line one\nline two\nline three\nline four\n");
    repo.commit_all("first");
    repo.git(&["mv", "old.txt", "new.txt"]);
    repo.write("new.txt", "line one\nline TWO\nline three\nline four\n");
    let request = DiffRequest {
        old_path: Some(RelativePath::parse("old_path", "old.txt").expect("path")),
        ..working_request("new.txt")
    };

    let diff = reader(&repo).diff(&repo.work, &request).expect("diff");

    let text = diff_text(&diff);
    assert!(text.contains("-line two\n+line TWO\n"), "{text}");
}

#[test]
fn an_untracked_file_diff_is_built_from_its_bytes() {
    let repo = Repo::new();
    repo.commit_all("first");
    repo.write("fresh.txt", "alpha\nbeta");
    repo.write("blob.bin", [1_u8, 0, 2]);
    repo.write("empty.txt", "");

    let fresh = reader(&repo)
        .diff(&repo.work, &working_request("fresh.txt"))
        .expect("diff");
    let blob = reader(&repo)
        .diff(&repo.work, &working_request("blob.bin"))
        .expect("diff");
    let empty = reader(&repo)
        .diff(&repo.work, &working_request("empty.txt"))
        .expect("diff");

    assert_eq!(
        diff_text(&fresh),
        "@@ -0,0 +1,2 @@\n+alpha\n+beta\n\\ No newline at end of file\n"
    );
    assert_eq!(blob.state, DiffState::Binary);
    assert_eq!(empty.state, DiffState::None);
}

#[test]
fn a_binary_change_is_binary_and_a_huge_diff_is_too_large() {
    let repo = Repo::new();
    repo.write("blob.bin", [1_u8, 0, 2]);
    repo.write("big.txt", "start\n");
    repo.commit_all("first");
    repo.write("blob.bin", [3_u8, 0, 4]);
    let big_lines = "a line of text that adds up\n".repeat(20_000);
    repo.write("big.txt", format!("start\n{big_lines}"));

    let blob = reader(&repo)
        .diff(&repo.work, &working_request("blob.bin"))
        .expect("diff");
    let big = reader(&repo)
        .diff(&repo.work, &working_request("big.txt"))
        .expect("diff");

    assert_eq!(blob.state, DiffState::Binary);
    assert_eq!((big.state, big.bytes), (DiffState::TooLarge, None));
    assert!(big.size.is_some_and(|size| size > 524_288));
}

#[test]
fn the_history_pages_by_fifty_and_counts_only_commits_inside_the_root() {
    let repo = Repo::new();
    fast_commits(&repo, &[("in", 120), ("out", 10)]);
    let root = repo.path("in");
    let reader = reader(&repo);

    let first = reader.history(&root, 0).expect("page 1");
    let second = reader.history(&root, 50).expect("page 2");
    let last = reader.history(&root, 100).expect("page 3");
    let beyond = reader.history(&root, 5_000).expect("beyond");

    assert_eq!((first.commits.len(), first.more), (50, true));
    assert_eq!((second.commits.len(), second.more), (50, true));
    assert_eq!((last.commits.len(), last.more), (20, false));
    assert!(beyond.commits.is_empty() && !beyond.more);
    assert_eq!(first.commits[0].subject, "change in 119");
    assert_eq!(last.commits[19].subject, "change in 0");
    assert!(
        first
            .commits
            .iter()
            .all(|commit| commit.subject.starts_with("change in"))
    );
    assert_eq!(first.commits[0].id.len(), 40);
    assert_eq!(first.commits[0].author, "T");
    assert_eq!(first.commits[0].time.as_unix_millis(), 1_700_000_119_000);
}

#[test]
fn a_long_subject_is_cut_to_200_characters() {
    let repo = Repo::new();
    repo.write("a.txt", "a\n");
    repo.commit_all(&"é".repeat(300));

    let history = reader(&repo).history(&repo.work, 0).expect("history");

    assert_eq!(
        history.commits[0].subject.chars().count(),
        MAX_SUBJECT_CHARS
    );
}

#[test]
fn a_commit_shows_its_message_and_its_files_against_the_first_parent() {
    let repo = Repo::new();
    repo.write("a.txt", "a\n");
    repo.write("sub/b.txt", "b\n");
    repo.commit_all("first");
    repo.write("a.txt", "a2\n");
    repo.write("sub/b.txt", "b2\n");
    repo.write("sub/c.txt", "c\n");
    repo.commit_all("second\n\nthe body of the message");
    let id = head_commit(&repo);

    let whole = reader(&repo).commit(&repo.work, &id).expect("commit");
    let inside = reader(&repo)
        .commit(&repo.path("sub"), &id)
        .expect("commit");

    assert_eq!(whole.id, id);
    assert_eq!(whole.parents.len(), 1);
    assert_eq!(whole.author, "Test");
    assert_eq!(whole.email, "test@example.com");
    assert_eq!(whole.message, "second\n\nthe body of the message");
    assert_eq!(
        paths(&whole.files),
        [
            ("a.txt", ChangeKind::Modified),
            ("sub/b.txt", ChangeKind::Modified),
            ("sub/c.txt", ChangeKind::Added),
        ]
    );
    assert_eq!(
        paths(&inside.files),
        [
            ("b.txt", ChangeKind::Modified),
            ("c.txt", ChangeKind::Added)
        ]
    );
}

#[test]
fn a_root_commit_shows_every_file_as_added_and_diffs_against_nothing() {
    let repo = Repo::new();
    repo.write("a.txt", "a\n");
    repo.commit_all("root");
    let id = head_commit(&repo);
    let request = DiffRequest {
        commit: Some(id.clone()),
        ..working_request("a.txt")
    };

    let detail = reader(&repo).commit(&repo.work, &id).expect("commit");
    let diff = reader(&repo).diff(&repo.work, &request).expect("diff");

    assert_eq!(detail.parents, Vec::<String>::new());
    assert_eq!(paths(&detail.files), [("a.txt", ChangeKind::Added)]);
    assert_eq!(diff_text(&diff), "@@ -0,0 +1 @@\n+a\n");
}

#[test]
fn a_merge_commit_is_compared_with_its_first_parent() {
    let repo = Repo::new();
    repo.write("base.txt", "base\n");
    repo.commit_all("base");
    repo.git(&["checkout", "--quiet", "-b", "side"]);
    repo.write("side.txt", "side\n");
    repo.commit_all("side work");
    repo.git(&["checkout", "--quiet", "main"]);
    repo.write("main.txt", "main\n");
    repo.commit_all("main work");
    repo.git(&["merge", "--quiet", "--no-ff", "-m", "merge side", "side"]);
    let id = head_commit(&repo);
    let request = DiffRequest {
        commit: Some(id.clone()),
        ..working_request("side.txt")
    };

    let detail = reader(&repo).commit(&repo.work, &id).expect("commit");
    let diff = reader(&repo).diff(&repo.work, &request).expect("diff");

    assert_eq!(detail.parents.len(), 2);
    assert_eq!(paths(&detail.files), [("side.txt", ChangeKind::Added)]);
    assert_eq!(diff_text(&diff), "@@ -0,0 +1 @@\n+side\n");
}

#[test]
fn an_id_that_is_not_a_commit_of_the_repository_is_invalid() {
    let repo = Repo::new();
    repo.write("a.txt", "a\n");
    repo.commit_all("first");
    let tree = repo.git(&["rev-parse", "HEAD^{tree}"]).trim().to_owned();
    let unknown = "0123456789abcdef0123456789abcdef01234567";
    let request = |commit: &str| DiffRequest {
        commit: Some(commit.to_owned()),
        ..working_request("a.txt")
    };

    for id in [tree.as_str(), unknown] {
        assert_eq!(
            reader(&repo).commit(&repo.work, id),
            Err(FileViewError::Invalid(NOT_A_COMMIT.to_owned())),
            "{id}"
        );
        assert_eq!(
            reader(&repo).diff(&repo.work, &request(id)),
            Err(FileViewError::Invalid(NOT_A_COMMIT.to_owned())),
            "{id}"
        );
    }
}

#[test]
fn a_huge_message_is_cut_at_64_kib() {
    let repo = Repo::new();
    repo.write("a.txt", "a\n");
    repo.write("message.txt", format!("subject\n\n{}", "é".repeat(60_000)));
    repo.git(&["add", "--all"]);
    repo.git(&["commit", "--quiet", "-F", "message.txt"]);
    let id = head_commit(&repo);

    let detail = reader(&repo).commit(&repo.work, &id).expect("commit");

    assert!(detail.message.len() <= MAX_MESSAGE_BYTES);
    assert!(detail.message.len() > MAX_MESSAGE_BYTES - 4);
    assert!(detail.message.starts_with("subject"));
}
