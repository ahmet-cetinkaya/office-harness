// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs;
use std::process::Command;

use kernel::AgentId;
use tempfile::TempDir;

use super::*;

/// A repository with `a.txt` and `sub/b.txt` on `main`, a home folder that
/// holds no git configuration, and the office's worktrees folder.
struct Fixture {
    _temp: TempDir,
    home: PathBuf,
    repository: PathBuf,
    root: PathBuf,
    worktrees: GitWorktrees,
}

fn fixture() -> Fixture {
    let temp = TempDir::new().expect("temp folder");
    let home = temp.path().join("home");
    let repository = temp.path().join("repo");
    let root = temp.path().join("data").join("worktrees");
    fs::create_dir_all(&home).expect("home");
    fs::create_dir_all(repository.join("sub")).expect("repository");
    git(
        &home,
        &repository,
        &["init", "--quiet", "--initial-branch=main"],
    );
    fs::write(repository.join("a.txt"), "a\n").expect("file");
    fs::write(repository.join("sub").join("b.txt"), "b\n").expect("file");
    commit_all(&home, &repository, "first");
    let worktrees = GitWorktrees::with_runner(root.clone(), Some(runner(&home)));
    Fixture {
        _temp: temp,
        home,
        repository,
        root,
        worktrees,
    }
}

fn test_environment(home: &Path) -> Environment {
    let mut environment: Environment = std::env::vars().collect();
    for name in ["HOME", "XDG_CONFIG_HOME", "USERPROFILE"] {
        environment.insert(name.to_owned(), home.to_string_lossy().into_owned());
    }
    environment
}

fn runner(home: &Path) -> GitRunner {
    let environment = test_environment(home);
    let program = find_program("git", &environment, home).expect("git is installed");
    GitRunner::new(program, &environment)
}

/// Runs the real git for the fixture, with no user or system configuration.
fn git(home: &Path, folder: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "-C",
        ])
        .arg(folder)
        .args(arguments)
        .envs(test_environment(home))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn commit_all(home: &Path, folder: &Path, message: &str) {
    git(home, folder, &["add", "--all"]);
    git(
        home,
        folder,
        &["commit", "--quiet", "--allow-empty", "-m", message],
    );
}

fn branches(fixture: &Fixture) -> String {
    git(&fixture.home, &fixture.repository, &["branch", "--list"])
}

fn worktree_records(fixture: &Fixture) -> String {
    git(
        &fixture.home,
        &fixture.repository,
        &["worktree", "list", "--porcelain"],
    )
}

fn create(fixture: &Fixture, folder: &Path, name: &str) -> Result<CreatedWorktree, WorktreeError> {
    fixture.worktrees.create(&WorktreeCreate {
        folder,
        agent_id: AgentId::generate(),
        agent_name: name,
        base: None,
        branch_name: None,
    })
}

fn created(fixture: &Fixture, name: &str) -> CreatedWorktree {
    create(fixture, &fixture.repository, name).expect("created")
}

fn assert_nothing_left(fixture: &Fixture) {
    assert!(
        !branches(fixture).contains("office-harness/"),
        "{}",
        branches(fixture)
    );
    assert_eq!(worktree_records(fixture).matches("worktree ").count(), 1);
    let leftovers: Vec<_> = fs::read_dir(&fixture.root)
        .into_iter()
        .flatten()
        .flatten()
        .flat_map(|entry| fs::read_dir(entry.path()).into_iter().flatten().flatten())
        .map(|entry| entry.path())
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[test]
fn info_in_a_repository_names_the_branch_and_the_free_branch_name() {
    let fixture = fixture();
    let info = fixture
        .worktrees
        .info(&fixture.repository, "Lead Dev")
        .expect("info")
        .expect("in a repository");
    assert_eq!(info.base, "main");
    assert!(info.base_is_branch);
    assert_eq!(info.branches, vec!["main".to_owned()]);
    assert_eq!(info.branch_name, "office-harness/lead-dev");
    assert_eq!(info.subfolder, "");
}

#[test]
fn info_in_a_subfolder_reads_the_prefix_also_through_a_symbolic_link() {
    let fixture = fixture();
    let info = fixture
        .worktrees
        .info(&fixture.repository.join("sub"), "x")
        .expect("info")
        .expect("in a repository");
    assert_eq!(info.subfolder, "sub/");
    #[cfg(unix)]
    {
        let link = fixture.home.join("link");
        std::os::unix::fs::symlink(&fixture.repository, &link).expect("link");
        let info = fixture
            .worktrees
            .info(&link.join("sub"), "x")
            .expect("info")
            .expect("in a repository");
        assert_eq!(info.subfolder, "sub/");
    }
}

#[test]
fn info_when_detached_gives_the_full_commit_id() {
    let fixture = fixture();
    let commit = git(&fixture.home, &fixture.repository, &["rev-parse", "HEAD"]);
    git(
        &fixture.home,
        &fixture.repository,
        &["checkout", "--quiet", "--detach"],
    );
    let info = fixture
        .worktrees
        .info(&fixture.repository, "x")
        .expect("info")
        .expect("in a repository");
    assert_eq!(info.base, commit.trim());
    assert!(!info.base_is_branch);
}

#[test]
fn info_outside_a_repository_or_without_git_is_not_in_a_repository() {
    let fixture = fixture();
    assert_eq!(fixture.worktrees.info(&fixture.home, "x"), Ok(None));
    let without_git = GitWorktrees::with_runner(fixture.root.clone(), None);
    assert_eq!(without_git.info(&fixture.repository, "x"), Ok(None));
    let missing = GitRunner::new(
        fixture.home.join("no-git-here"),
        &test_environment(&fixture.home),
    );
    let missing = GitWorktrees::with_runner(fixture.root.clone(), Some(missing));
    assert_eq!(missing.info(&fixture.repository, "x"), Ok(None));
}

#[test]
fn a_taken_branch_name_moves_to_the_next_free_one() {
    let fixture = fixture();
    git(
        &fixture.home,
        &fixture.repository,
        &["branch", "office-harness/dev"],
    );
    let info = fixture
        .worktrees
        .info(&fixture.repository, "dev")
        .expect("info")
        .expect("in a repository");
    assert_eq!(info.branch_name, "office-harness/dev-2");
    let error = fixture
        .worktrees
        .create(&WorktreeCreate {
            folder: &fixture.repository,
            agent_id: AgentId::generate(),
            agent_name: "dev",
            base: None,
            branch_name: Some("office-harness/dev"),
        })
        .expect_err("taken");
    assert_eq!(
        error,
        WorktreeError::Conflict(
            "the branch office-harness/dev exists; the next free name is office-harness/dev-2"
                .to_owned()
        )
    );
}

#[test]
fn a_base_outside_the_listed_branches_is_refused_before_git_runs() {
    let fixture = fixture();
    for base in ["-x", "--help", "nope", "HEAD"] {
        let error = fixture
            .worktrees
            .create(&WorktreeCreate {
                folder: &fixture.repository,
                agent_id: AgentId::generate(),
                agent_name: "x",
                base: Some(base),
                branch_name: None,
            })
            .expect_err("refused");
        assert_eq!(
            error,
            WorktreeError::Invalid(format!("base is not a branch of the repository: {base}"))
        );
    }
    assert_nothing_left(&fixture);
}

#[test]
fn a_branch_name_without_the_prefix_is_refused() {
    let fixture = fixture();
    let error = fixture
        .worktrees
        .create(&WorktreeCreate {
            folder: &fixture.repository,
            agent_id: AgentId::generate(),
            agent_name: "x",
            base: None,
            branch_name: Some("main2"),
        })
        .expect_err("refused");
    assert_eq!(error, WorktreeError::Invalid(BRANCH_NAME_SHAPE.to_owned()));
}

#[test]
fn create_and_remove_leave_nothing_and_the_git_configuration_unchanged() {
    let fixture = fixture();
    let config = fixture.repository.join(".git").join("config");
    let before = fs::read(&config).expect("config");
    let made = created(&fixture, "Dev");
    assert_eq!(made.worktree.branch, "office-harness/dev");
    assert_eq!(made.worktree.base, "main");
    assert_eq!(made.working_folder, made.worktree.folder);
    assert!(made.worktree.folder.join("a.txt").is_file());
    assert!(made.worktree.folder.starts_with(&fixture.root));
    let status = fixture.worktrees.status(&made.worktree);
    assert_eq!(status.changed_files, 0);
    assert!(status.reachable && status.base_exists);
    let commit = status.deletable_branch_commit().expect("deletable");
    fixture
        .worktrees
        .remove(&made.worktree, false)
        .expect("removed");
    fixture
        .worktrees
        .delete_branch(&made.worktree, commit)
        .expect("branch deleted");
    assert_nothing_left(&fixture);
    assert_eq!(fs::read(&config).expect("config"), before);
}

#[test]
fn a_subfolder_create_starts_the_agent_in_the_same_subfolder() {
    let fixture = fixture();
    let made = create(&fixture, &fixture.repository.join("sub"), "x").expect("created");
    assert_eq!(made.working_folder, made.worktree.folder.join("sub"));
    assert!(made.working_folder.join("b.txt").is_file());
}

#[test]
fn a_subfolder_untracked_in_the_base_refuses_the_create_and_leaves_nothing() {
    let fixture = fixture();
    let untracked = fixture.repository.join("new");
    fs::create_dir_all(&untracked).expect("folder");
    let error = create(&fixture, &untracked, "x").expect_err("refused");
    assert_eq!(
        error,
        WorktreeError::Invalid("the folder new does not exist in main".to_owned())
    );
    assert_nothing_left(&fixture);
}

#[test]
fn the_data_folder_inside_the_repository_is_refused() {
    let fixture = fixture();
    let inside =
        GitWorktrees::with_runner(fixture.repository.join("data"), Some(runner(&fixture.home)));
    let error = inside
        .create(&WorktreeCreate {
            folder: &fixture.repository,
            agent_id: AgentId::generate(),
            agent_name: "x",
            base: None,
            branch_name: None,
        })
        .expect_err("refused");
    assert_eq!(error, WorktreeError::Invalid(DATA_FOLDER_INSIDE.to_owned()));
}

#[cfg(unix)]
#[test]
fn an_unwritable_worktrees_folder_fails_the_create_and_leaves_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = fixture();
    let agent_id = AgentId::generate();
    let folder = fixture.worktrees.folder_for(
        Path::new(
            git(
                &fixture.home,
                &fixture.repository,
                &["rev-parse", "--show-toplevel"],
            )
            .trim(),
        ),
        &agent_id.to_string(),
    );
    let parent = folder.parent().expect("parent");
    fs::create_dir_all(parent).expect("parent");
    fs::set_permissions(parent, fs::Permissions::from_mode(0o555)).expect("read-only");
    let error = fixture.worktrees.create(&WorktreeCreate {
        folder: &fixture.repository,
        agent_id,
        agent_name: "x",
        base: None,
        branch_name: None,
    });
    fs::set_permissions(parent, fs::Permissions::from_mode(0o755)).expect("writable");
    assert!(matches!(error, Err(WorktreeError::Failed(_))), "{error:?}");
    assert_nothing_left(&fixture);
}

#[cfg(unix)]
#[test]
fn a_checkout_that_times_out_leaves_nothing() {
    let fixture = fixture();
    git(
        &fixture.home,
        &fixture.repository,
        &["config", "filter.slow.smudge", "sleep 30; cat"],
    );
    fs::write(
        fixture.repository.join(".gitattributes"),
        "*.txt filter=slow\n",
    )
    .expect("file");
    commit_all(&fixture.home, &fixture.repository, "slow");
    let slow = GitWorktrees::with_runner(
        fixture.root.clone(),
        Some(
            runner(&fixture.home)
                .with_timeouts(READ_TIMEOUT_FOR_TESTS, std::time::Duration::from_secs(1)),
        ),
    );
    let error = slow
        .create(&WorktreeCreate {
            folder: &fixture.repository,
            agent_id: AgentId::generate(),
            agent_name: "x",
            base: None,
            branch_name: None,
        })
        .expect_err("timed out");
    assert_eq!(
        error,
        WorktreeError::Failed(
            "the worktree could not be created: git did not finish in time".to_owned()
        )
    );
    assert_nothing_left(&fixture);
}

#[cfg(unix)]
const READ_TIMEOUT_FOR_TESTS: std::time::Duration = std::time::Duration::from_secs(10);

#[cfg(unix)]
#[test]
fn neither_a_hook_file_nor_a_configured_hook_runs() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = fixture();
    let file_marker = fixture.home.join("file-hook-ran");
    let hook = fixture
        .repository
        .join(".git")
        .join("hooks")
        .join("post-checkout");
    fs::write(
        &hook,
        format!("#!/bin/sh\ntouch '{}'\n", file_marker.display()),
    )
    .expect("hook");
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).expect("runnable");
    // Hooks named in the configuration, which newer git runs on its own.
    let config_marker = fixture.home.join("configured-hook-ran");
    let command = format!("touch '{}'", config_marker.display());
    for (key, value) in [
        ("hook.marker.command", command.as_str()),
        ("hook.marker.event", "post-checkout"),
        ("hook.marker.event", "reference-transaction"),
    ] {
        git(
            &fixture.home,
            &fixture.repository,
            &["config", "--add", key, value],
        );
    }
    let made = created(&fixture, "x");
    let status = fixture.worktrees.status(&made.worktree);
    let commit = status.deletable_branch_commit().expect("deletable");
    assert!(!file_marker.exists());
    assert!(!config_marker.exists());
    fixture
        .worktrees
        .remove(&made.worktree, false)
        .expect("removed");
    fixture
        .worktrees
        .delete_branch(&made.worktree, commit)
        .expect("deleted");
    assert!(!file_marker.exists());
    assert!(!config_marker.exists());
}

#[test]
fn the_status_counts_every_untracked_file_and_skips_ignored_ones() {
    let fixture = fixture();
    let made = created(&fixture, "x");
    let folder = &made.worktree.folder;
    git(
        &fixture.home,
        folder,
        &["config", "status.showUntrackedFiles", "no"],
    );
    fs::create_dir_all(folder.join("new")).expect("folder");
    for name in ["1", "2", "3"] {
        fs::write(folder.join("new").join(name), name).expect("file");
    }
    assert_eq!(fixture.worktrees.status(&made.worktree).changed_files, 3);
    fs::write(folder.join(".gitignore"), "ignored\n").expect("file");
    fs::write(folder.join("ignored"), "x").expect("file");
    fs::write(folder.join("a.txt"), "changed\n").expect("file");
    fs::remove_file(folder.join("sub").join("b.txt")).expect("deleted");
    // 3 untracked + .gitignore + a.txt changed + b.txt deleted.
    assert_eq!(fixture.worktrees.status(&made.worktree).changed_files, 6);
}

#[test]
fn the_hash_changes_when_one_file_is_reverted_and_another_added() {
    let fixture = fixture();
    let made = created(&fixture, "x");
    let folder = &made.worktree.folder;
    // The checkout's own bytes: git may have written other line endings.
    let original = fs::read(folder.join("a.txt")).expect("file");
    fs::write(folder.join("a.txt"), "changed\n").expect("file");
    let first = fixture.worktrees.status(&made.worktree);
    fs::write(folder.join("a.txt"), original).expect("reverted");
    fs::write(folder.join("c.txt"), "c\n").expect("added");
    let second = fixture.worktrees.status(&made.worktree);
    assert_eq!(first.changed_files, second.changed_files);
    assert_ne!(first.status_hash, second.status_hash);
}

#[test]
fn the_hash_changes_when_an_already_changed_file_is_edited_again() {
    let fixture = fixture();
    let made = created(&fixture, "x");
    let file = made.worktree.folder.join("a.txt");
    fs::write(&file, "changed\n").expect("file");
    let first = fixture.worktrees.status(&made.worktree);
    fs::write(&file, "changed again, longer\n").expect("file");
    let second = fixture.worktrees.status(&made.worktree);
    assert_eq!(first.changed_files, second.changed_files);
    assert_ne!(first.status_hash, second.status_hash);
}

#[test]
fn a_renamed_file_counts_once() {
    let fixture = fixture();
    let made = created(&fixture, "x");
    git(
        &fixture.home,
        &made.worktree.folder,
        &["mv", "a.txt", "renamed.txt"],
    );
    assert_eq!(fixture.worktrees.status(&made.worktree).changed_files, 1);
}

#[test]
fn own_commits_count_what_the_base_lacks_and_a_missing_base_keeps_the_branch() {
    let fixture = fixture();
    git(&fixture.home, &fixture.repository, &["branch", "topic"]);
    let made = fixture
        .worktrees
        .create(&WorktreeCreate {
            folder: &fixture.repository,
            agent_id: AgentId::generate(),
            agent_name: "x",
            base: Some("topic"),
            branch_name: None,
        })
        .expect("created");
    commit_all(&fixture.home, &made.worktree.folder, "own 1");
    commit_all(&fixture.home, &made.worktree.folder, "own 2");
    let status = fixture.worktrees.status(&made.worktree);
    assert_eq!(status.own_commits, Some(2));
    assert!(status.deletable_branch_commit().is_none());
    git(
        &fixture.home,
        &fixture.repository,
        &["branch", "-D", "topic"],
    );
    let status = fixture.worktrees.status(&made.worktree);
    assert!(!status.base_exists);
    assert!(status.deletable_branch_commit().is_none());
}

#[test]
fn own_commits_that_cannot_be_counted_keep_the_branch() {
    let fixture = fixture();
    let made = created(&fixture, "x");
    commit_all(&fixture.home, &made.worktree.folder, "own");
    let commit = git(&fixture.home, &made.worktree.folder, &["rev-parse", "HEAD"]);
    let (directory, file) = commit.trim().split_at(2);
    let objects = fixture.repository.join(".git").join("objects");
    fs::remove_file(objects.join(directory).join(file)).expect("loose object");
    let status = fixture.worktrees.status(&made.worktree);
    assert_eq!(status.own_commits, None);
    assert!(status.deletable_branch_commit().is_none());
}

#[test]
fn info_in_a_linked_worktree_names_the_main_repository() {
    let fixture = fixture();
    let linked = fixture.home.join("linked");
    git(
        &fixture.home,
        &fixture.repository,
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "side",
            &linked.to_string_lossy(),
        ],
    );
    let info = fixture
        .worktrees
        .info(&linked.join("sub"), "x")
        .expect("info")
        .expect("in a repository");
    assert_eq!(
        info.top.canonicalize().expect("top"),
        fixture.repository.canonicalize().expect("repository")
    );
    assert_eq!(info.base, "side");
    assert_eq!(info.subfolder, "sub/");
}

#[test]
fn a_branch_checked_out_elsewhere_is_not_deleted() {
    let fixture = fixture();
    let made = created(&fixture, "x");
    let status = fixture.worktrees.status(&made.worktree);
    let commit = status.deletable_branch_commit().expect("deletable");
    let error = fixture.worktrees.delete_branch(&made.worktree, commit);
    assert!(matches!(error, Err(WorktreeError::Failed(_))), "{error:?}");
    assert!(branches(&fixture).contains("office-harness/x"));
}

#[test]
fn a_branch_that_moved_since_its_count_is_not_deleted() {
    let fixture = fixture();
    let made = created(&fixture, "x");
    let status = fixture.worktrees.status(&made.worktree);
    let counted = status
        .deletable_branch_commit()
        .expect("deletable")
        .to_owned();
    commit_all(&fixture.home, &made.worktree.folder, "late");
    fixture
        .worktrees
        .remove(&made.worktree, true)
        .expect("removed");
    let error = fixture.worktrees.delete_branch(&made.worktree, &counted);
    assert!(matches!(error, Err(WorktreeError::Failed(_))), "{error:?}");
    assert!(branches(&fixture).contains("office-harness/x"));
}

#[test]
fn a_worktree_that_became_dirty_is_refused_by_the_plain_remove() {
    let fixture = fixture();
    // Git's own clean check would not see the new file with this setting.
    git(
        &fixture.home,
        &fixture.repository,
        &["config", "status.showUntrackedFiles", "no"],
    );
    let made = created(&fixture, "x");
    fs::write(made.worktree.folder.join("new.txt"), "x").expect("file");
    let error = fixture.worktrees.remove(&made.worktree, false);
    assert!(matches!(error, Err(WorktreeError::Failed(_))), "{error:?}");
    assert!(made.worktree.folder.join("new.txt").is_file());
}

#[test]
fn a_forced_remove_of_a_missing_folder_clears_only_its_own_record() {
    let fixture = fixture();
    let first = created(&fixture, "one");
    let second = created(&fixture, "two");
    fs::remove_dir_all(&first.worktree.folder).expect("gone");
    fs::remove_dir_all(&second.worktree.folder).expect("gone");
    let status = fixture.worktrees.status(&first.worktree);
    assert!(status.reachable && !status.folder_exists);
    fixture
        .worktrees
        .remove(&first.worktree, true)
        .expect("record cleared");
    let records = worktree_records(&fixture);
    // Git records the real path (`/private/var` on macOS), so compare the
    // canonical form of the folder that still exists.
    let kept_parent = second.worktree.folder.parent().expect("parent");
    let gone = kept_parent
        .canonicalize()
        .expect("parent exists")
        .join(first.worktree.folder.file_name().expect("name"));
    assert!(!records.contains(&*gone.to_string_lossy()), "{records}");
    assert_eq!(records.matches("worktree ").count(), 2, "{records}");
}

#[test]
fn a_folder_outside_the_worktrees_folder_is_never_removed() {
    let fixture = fixture();
    let made = created(&fixture, "x");
    let repository_folder = made.worktree.folder.parent().expect("parent");
    for folder in [
        fixture.repository.clone(),
        fixture.root.join("..").join("repo"),
        fixture.root.clone(),
        repository_folder.to_path_buf(),
        made.worktree.folder.join("sub"),
    ] {
        let outside = AgentWorktree {
            folder,
            ..made.worktree.clone()
        };
        assert!(matches!(
            fixture.worktrees.remove(&outside, true),
            Err(WorktreeError::Invalid(_))
        ));
    }
    assert!(fixture.repository.join("a.txt").is_file());
}

#[test]
fn a_missing_repository_reads_as_unreachable() {
    let fixture = fixture();
    let made = created(&fixture, "x");
    let moved = fixture.home.join("moved");
    fs::rename(&fixture.repository, &moved).expect("moved");
    let status = fixture.worktrees.status(&made.worktree);
    assert!(!status.reachable);
}

#[test]
fn a_relative_worktrees_folder_is_made_absolute() {
    let worktrees = GitWorktrees::with_runner(PathBuf::from("relative-data"), None);
    assert!(worktrees.root.is_absolute());
    assert!(
        worktrees
            .folder_for(Path::new("/r"), "12345678-x")
            .is_absolute()
    );
}

#[test]
fn without_git_on_the_login_path_the_folder_is_not_in_a_repository() {
    let fixture = fixture();
    let empty = fixture.home.join("empty-bin");
    fs::create_dir_all(&empty).expect("folder");
    let environment = [("PATH".to_owned(), empty.to_string_lossy().into_owned())]
        .into_iter()
        .collect();
    let worktrees = GitWorktrees::new(
        fixture.root.clone(),
        Arc::new(LoginEnvironment::fixed(environment)),
        Environment::new(),
    );
    assert_eq!(worktrees.info(&fixture.repository, "x"), Ok(None));
}

#[test]
fn a_branch_named_like_a_commit_id_is_refused_as_base() {
    let fixture = fixture();
    let hex = "a".repeat(40);
    git(&fixture.home, &fixture.repository, &["branch", &hex]);
    let error = fixture
        .worktrees
        .create(&WorktreeCreate {
            folder: &fixture.repository,
            agent_id: AgentId::generate(),
            agent_name: "x",
            base: Some(&hex),
            branch_name: None,
        })
        .expect_err("refused");
    assert!(matches!(error, WorktreeError::Invalid(_)), "{error:?}");
    assert_nothing_left(&fixture);
}

#[test]
fn a_worktree_without_its_git_link_reads_as_unreachable() {
    let fixture = fixture();
    let made = created(&fixture, "x");
    fs::remove_file(made.worktree.folder.join(".git")).expect("link removed");
    let status = fixture.worktrees.status(&made.worktree);
    assert!(status.folder_exists && !status.reachable);
}

#[cfg(unix)]
#[test]
fn a_subfolder_that_is_a_link_out_of_the_worktree_in_the_base_is_refused() {
    let fixture = fixture();
    git(
        &fixture.home,
        &fixture.repository,
        &["checkout", "--quiet", "-b", "evil"],
    );
    git(
        &fixture.home,
        &fixture.repository,
        &["rm", "--quiet", "-r", "sub"],
    );
    std::os::unix::fs::symlink("../..", fixture.repository.join("sub")).expect("link");
    commit_all(&fixture.home, &fixture.repository, "link");
    git(
        &fixture.home,
        &fixture.repository,
        &["checkout", "--quiet", "main"],
    );
    let error = fixture
        .worktrees
        .create(&WorktreeCreate {
            folder: &fixture.repository.join("sub"),
            agent_id: AgentId::generate(),
            agent_name: "x",
            base: Some("evil"),
            branch_name: None,
        })
        .expect_err("refused");
    assert_eq!(
        error,
        WorktreeError::Invalid("the folder sub does not exist in evil".to_owned())
    );
    assert_nothing_left(&fixture);
}
