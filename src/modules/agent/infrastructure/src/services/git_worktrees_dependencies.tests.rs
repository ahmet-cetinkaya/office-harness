// SPDX-License-Identifier: AGPL-3.0-or-later

//! The dependency links of a new own worktree (0021), through the real git.

use super::*;

const SLASH_RULE: &str = "node_modules/\n";
const NAME_RULE: &str = "node_modules\n";
const WORKSPACE: &str = "web";

/// Tracks `web/package.json` and the ignore rule, and installs packages in
/// the base checkout at the top and in `web`.
fn install(fixture: &Fixture, rule: &str) {
    fs::write(fixture.repository.join(".gitignore"), rule).expect("ignore file");
    let workspace = fixture.repository.join(WORKSPACE);
    fs::create_dir_all(&workspace).expect("workspace");
    fs::write(workspace.join("package.json"), "{}\n").expect("package file");
    commit_all(&fixture.home, &fixture.repository, "packages");
    write_package(&fixture.repository, "pkg", "top\n");
    write_package(&workspace, "lib", "web\n");
    let top = fixture.repository.join("node_modules");
    for entry in [".bin", ".vite", ".cache"] {
        fs::create_dir_all(top.join(entry)).expect("dot entry");
    }
}

fn write_package(place: &Path, name: &str, text: &str) {
    let package = place.join("node_modules").join(name);
    fs::create_dir_all(&package).expect("package");
    fs::write(package.join("index.js"), text).expect("package file");
}

fn create_linked(worktrees: &GitWorktrees, folder: &Path, is_on: bool) -> CreatedWorktree {
    worktrees
        .create(&WorktreeCreate {
            folder,
            agent_id: AgentId::generate(),
            agent_name: "deps",
            base: None,
            branch_name: None,
            link_dependencies: is_on,
        })
        .expect("created")
}

fn recording(fixture: &Fixture) -> (GitWorktrees, Arc<RecordedFailures>) {
    let failures = Arc::new(RecordedFailures::default());
    let worktrees = GitWorktrees::with_runner(fixture.root.clone(), Some(runner(&fixture.home)))
        .with_failures(Arc::clone(&failures) as Arc<dyn FailureReporter>);
    (worktrees, failures)
}

/// The reported reasons, without the storage prefix of the log line.
fn recorded(failures: &RecordedFailures) -> Vec<String> {
    let mut reasons: Vec<String> = failures
        .0
        .lock()
        .expect("lock")
        .iter()
        .map(|line| line.trim_start_matches("storage failure: ").to_owned())
        .collect();
    reasons.sort();
    reasons
}

/// The base checkout's installed packages at both places.
fn installed(fixture: &Fixture) -> Vec<(PathBuf, String)> {
    let mut found = listing(&fixture.repository.join("node_modules"));
    found.extend(listing(
        &fixture.repository.join(WORKSPACE).join("node_modules"),
    ));
    found
}

/// Every path below `folder`, links not followed, with its content or its
/// link target.
fn listing(folder: &Path) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    let mut pending = vec![folder.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(&next).expect("read").flatten() {
            let path = entry.path();
            let kind = entry.file_type().expect("type");
            let content = if kind.is_symlink() {
                format!("link {}", fs::read_link(&path).expect("link").display())
            } else if kind.is_dir() {
                pending.push(path.clone());
                "folder".to_owned()
            } else {
                fs::read_to_string(&path).expect("text")
            };
            found.push((path, content));
        }
    }
    found.sort();
    found
}

fn porcelain(fixture: &Fixture, folder: &Path) -> String {
    git(
        &fixture.home,
        folder,
        &["status", "--porcelain", "--untracked-files=all"],
    )
}

fn is_link(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink())
}

#[cfg(unix)]
#[test]
fn packages_are_linked_at_the_top_and_beside_each_tracked_package_file() {
    for rule in [SLASH_RULE, NAME_RULE] {
        let fixture = fixture();
        install(&fixture, rule);

        let made = create_linked(&fixture.worktrees, &fixture.repository, true);

        let folder = &made.worktree.folder;
        let top = folder.join("node_modules");
        assert!(fs::symlink_metadata(&top).expect("top").is_dir(), "{rule}");
        assert!(
            is_link(&top.join("pkg")) && is_link(&top.join(".bin")),
            "{rule}"
        );
        // Tool caches stay unlinked, so no two builds write one cache.
        assert!(!top.join(".vite").exists() && !top.join(".cache").exists());
        assert_eq!(
            fs::read_to_string(top.join("pkg").join("index.js")).expect("top package"),
            "top\n"
        );
        let web = folder.join(WORKSPACE).join("node_modules").join("lib");
        assert_eq!(
            fs::read_to_string(web.join("index.js")).expect("web package"),
            "web\n"
        );
        assert_eq!(porcelain(&fixture, folder), "", "{rule}");
        assert_eq!(fixture.worktrees.status(&made.worktree).changed_files, 0);
    }
}

#[cfg(unix)]
#[test]
fn staging_everything_stages_no_link() {
    let fixture = fixture();
    install(&fixture, SLASH_RULE);
    let made = create_linked(&fixture.worktrees, &fixture.repository, true);

    git(&fixture.home, &made.worktree.folder, &["add", "--all"]);

    let staged = git(
        &fixture.home,
        &made.worktree.folder,
        &["diff", "--cached", "--name-only"],
    );
    assert_eq!(staged, "");
}

#[cfg(unix)]
#[test]
fn remove_and_discard_unlink_first_and_leave_the_base_checkout_unchanged() {
    let fixture = fixture();
    install(&fixture, SLASH_RULE);
    let before = installed(&fixture);
    let removed = create_linked(&fixture.worktrees, &fixture.repository, true);
    let discarded = create_linked(&fixture.worktrees, &fixture.repository, true);

    fixture
        .worktrees
        .remove(&removed.worktree, false)
        .expect("removed");
    fixture.worktrees.discard(&discarded.worktree);

    assert!(!removed.worktree.folder.exists());
    assert!(!discarded.worktree.folder.exists());
    assert_eq!(installed(&fixture), before);
}

#[cfg(unix)]
#[test]
fn the_agents_own_link_and_files_are_left_to_the_removal_and_never_followed() {
    let fixture = fixture();
    install(&fixture, SLASH_RULE);
    let outside = fixture.root.parent().expect("data").join("outside");
    fs::create_dir_all(&outside).expect("outside");
    fs::write(outside.join("keep.txt"), "keep\n").expect("outside file");
    let made = create_linked(&fixture.worktrees, &fixture.repository, true);
    let top = made.worktree.folder.join("node_modules");
    std::os::unix::fs::symlink(&outside, top.join("own")).expect("agent link");
    fs::write(top.join("real.txt"), "real\n").expect("agent file");
    let before = installed(&fixture);

    fixture
        .worktrees
        .remove(&made.worktree, true)
        .expect("removed");

    assert!(!made.worktree.folder.exists());
    assert_eq!(
        fs::read_to_string(outside.join("keep.txt")).expect("kept"),
        "keep\n"
    );
    assert_eq!(installed(&fixture), before);
}

#[test]
fn nothing_is_linked_while_the_setting_is_off() {
    let fixture = fixture();
    install(&fixture, SLASH_RULE);

    let made = create_linked(&fixture.worktrees, &fixture.repository, false);

    assert!(!made.worktree.folder.join("node_modules").exists());
    assert!(
        !made
            .worktree
            .folder
            .join(WORKSPACE)
            .join("node_modules")
            .exists()
    );
}

#[cfg(unix)]
#[test]
fn a_place_without_an_ignored_free_folder_gets_nothing() {
    let fixture = fixture();
    // `tools` tracks a file at the place, and `web` has no ignore rule.
    let tools = fixture.repository.join("tools");
    fs::create_dir_all(&tools).expect("tools");
    fs::write(tools.join("package.json"), "{}\n").expect("package file");
    fs::write(tools.join("node_modules"), "tracked\n").expect("tracked file");
    commit_all(&fixture.home, &fixture.repository, "tools");
    install(&fixture, "/node_modules/\n");

    let made = create_linked(&fixture.worktrees, &fixture.repository, true);

    let folder = &made.worktree.folder;
    assert!(is_link(&folder.join("node_modules").join("pkg")));
    assert!(!folder.join(WORKSPACE).join("node_modules").exists());
    assert_eq!(
        fs::read_to_string(folder.join("tools").join("node_modules")).expect("tracked"),
        "tracked\n"
    );
    assert_eq!(porcelain(&fixture, folder), "");
}

#[cfg(unix)]
#[test]
fn links_that_lead_outside_the_base_checkout_are_refused_and_logged() {
    let fixture = fixture();
    install(&fixture, NAME_RULE);
    let outside = fixture.root.parent().expect("data").join("outside");
    fs::create_dir_all(&outside).expect("outside");
    let top = fixture.repository.join("node_modules");
    std::os::unix::fs::symlink(&outside, top.join("linked")).expect("package link");
    std::os::unix::fs::symlink(outside.join("gone"), top.join("broken")).expect("broken link");
    let web = fixture.repository.join(WORKSPACE).join("node_modules");
    fs::remove_dir_all(&web).expect("web packages");
    std::os::unix::fs::symlink(&outside, &web).expect("folder link");
    let (worktrees, failures) = recording(&fixture);

    let made = create_linked(&worktrees, &fixture.repository, true);

    let linked = made.worktree.folder.join("node_modules");
    assert!(is_link(&linked.join("pkg")));
    assert!(!linked.join("linked").exists() && !is_link(&linked.join("broken")));
    assert!(
        !made
            .worktree
            .folder
            .join(WORKSPACE)
            .join("node_modules")
            .exists()
    );
    assert_eq!(
        recorded(&failures),
        [
            "the dependency link node_modules/broken was not made: it leads outside the base \
             checkout or to nothing",
            "the dependency link node_modules/linked was not made: it leads outside the base \
             checkout or to nothing",
            "the dependency link web/node_modules was not made: it leads outside the base \
             checkout or to nothing",
        ]
    );
}

#[cfg(unix)]
#[test]
fn a_place_that_cannot_be_linked_is_logged_and_the_create_goes_on() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = fixture();
    install(&fixture, SLASH_RULE);
    let unreadable = fixture.repository.join(WORKSPACE).join("node_modules");
    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000)).expect("lock");
    let (worktrees, failures) = recording(&fixture);

    let made = create_linked(&worktrees, &fixture.repository, true);

    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o755)).expect("unlock");
    assert!(is_link(
        &made.worktree.folder.join("node_modules").join("pkg")
    ));
    assert!(
        !made
            .worktree
            .folder
            .join(WORKSPACE)
            .join("node_modules")
            .exists()
    );
    let reasons = recorded(&failures);
    assert_eq!(reasons.len(), 1, "{reasons:?}");
    assert!(
        reasons[0].starts_with("the dependency links of web/node_modules were not made: "),
        "{reasons:?}"
    );
}

#[cfg(windows)]
#[test]
fn no_link_is_made_on_windows() {
    let fixture = fixture();
    install(&fixture, SLASH_RULE);

    let made = create_linked(&fixture.worktrees, &fixture.repository, true);

    assert!(!made.worktree.folder.join("node_modules").exists());
}
