// SPDX-License-Identifier: AGPL-3.0-or-later

//! The agent's own branch and worktree through the system git program
//! (0010 D1 to D6).

use std::ffi::OsStr;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use agent_application::common::errors::PortError;
use agent_application::common::interfaces::{
    CreatedWorktree, FailureReporter, WorktreeCreate, WorktreeError, WorktreeInfo, WorktreeManager,
    WorktreeStatus,
};
use agent_application::common::worktree_link::is_linked_worktree;
use agent_domain::value_objects::{
    AgentWorktree, base_ref, free_branch_name, is_commit_id, is_office_branch_name,
};

use crate::services::dependency_links::{link_dependencies, unlink_dependencies};
use crate::services::git_runner::{
    GitFailure, GitRunner, LazyGitRunner, absolute, filter_switches,
};

const BRANCH_REF_PREFIX: &str = "refs/heads/";
const WORKTREE_RECORD_PREFIX: &str = "worktree ";
const BARE_RECORD: &str = "bare";
const BRANCH_RECORD_PREFIX: &str = "branch ";
const SHOW_UNTRACKED_FILES: &str = "status.showUntrackedFiles=normal";
/// Submodules count by their commit only, so git starts nothing inside one,
/// where their own configuration would apply.
const STATUS_ARGUMENTS: [&str; 5] = [
    "status",
    "--porcelain=v1",
    "--untracked-files=all",
    "--ignore-submodules=dirty",
    "-z",
];
const SHORT_ID_CHARS: usize = 8;
const REPOSITORY_HASH_CHARS: usize = 8;
/// `<repository>-<hash>/<short agent id>` below the worktrees folder.
const OWNED_DEPTH: usize = 2;
const NEEDS_REPOSITORY: &str = "own worktree needs a git repository";
const DATA_FOLDER_INSIDE: &str = "the office data folder is inside this repository";
/// The undo of a create removes even the locked worktree of an interrupted
/// `worktree add`.
const DISCARD_FORCE: usize = 2;
const BRANCH_NAME_SHAPE: &str = "branch_name must be office-harness/ and a name of a-z, 0-9 and -";

/// `WorktreeManager` over the system git program. Worktrees live under
/// `root`, which is outside every repository it serves.
pub struct GitWorktrees {
    root: PathBuf,
    runner: Arc<LazyGitRunner>,
    /// Hears what an undo leaves behind, which no answer can carry.
    failures: Option<Arc<dyn FailureReporter>>,
}

impl GitWorktrees {
    /// Runs git through `runner`, which the file view shares.
    #[must_use]
    pub fn new(
        root: PathBuf,
        runner: Arc<LazyGitRunner>,
        failures: Arc<dyn FailureReporter>,
    ) -> Self {
        Self {
            root: absolute(root),
            runner,
            failures: Some(failures),
        }
    }

    /// Uses `runner`, or no git at all for `None`; for tests.
    #[must_use]
    pub fn with_runner(root: PathBuf, runner: Option<GitRunner>) -> Self {
        Self {
            root: absolute(root),
            runner: Arc::new(LazyGitRunner::fixed(runner)),
            failures: None,
        }
    }

    /// Reports what an undo leaves behind to `failures`; for tests.
    #[must_use]
    pub fn with_failures(mut self, failures: Arc<dyn FailureReporter>) -> Self {
        self.failures = Some(failures);
        self
    }

    fn runner(&self) -> Option<&GitRunner> {
        self.runner.get()
    }

    fn require_runner(&self) -> Result<&GitRunner, WorktreeError> {
        self.runner()
            .ok_or_else(|| WorktreeError::Invalid(NEEDS_REPOSITORY.to_owned()))
    }

    /// `<root>/<repository folder name>-<hash>/<short agent id>` (D2).
    fn folder_for(&self, top: &Path, agent_id: &str) -> PathBuf {
        let name = top.file_name().map_or_else(
            || "repository".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        );
        let hash = hex_hash(top.as_os_str().as_encoded_bytes());
        self.root
            .join(format!("{name}-{}", &hash[..REPOSITORY_HASH_CHARS]))
            .join(agent_id.chars().take(SHORT_ID_CHARS).collect::<String>())
    }

    /// True when `folder` has the shape `folder_for` makes: exactly a
    /// repository folder and an agent folder below the worktrees folder,
    /// read lexically so a stored path with `..` cannot leave it, and never
    /// a whole repository's folder.
    fn is_owned(&self, folder: &Path) -> bool {
        folder.strip_prefix(&self.root).is_ok_and(|inside| {
            inside.components().count() == OWNED_DEPTH
                && inside
                    .components()
                    .all(|component| matches!(component, Component::Normal(_)))
        })
    }

    /// `is_owned`, and also after links resolve: the real folder that a
    /// delete of the whole tree would empty is still the office's own.
    fn is_really_owned(&self, folder: &Path) -> bool {
        let (Ok(real), Some(root)) = (folder.canonicalize(), canonical_existing(&self.root)) else {
            return false;
        };
        self.is_owned(folder)
            && real
                .strip_prefix(&root)
                .is_ok_and(|inside| inside.components().count() == OWNED_DEPTH)
    }

    /// Refuses a worktrees folder inside the repository, or inside any of
    /// its linked worktrees, before anything is written there (A1, PB-2).
    fn ensure_outside(&self, runner: &GitRunner, top: &Path) -> Result<(), WorktreeError> {
        let (Some(root), Ok(top)) = (canonical_existing(&self.root), top.canonicalize()) else {
            return Err(WorktreeError::Failed(
                "the worktrees folder cannot be read".to_owned(),
            ));
        };
        let recorded = worktree_folders(runner, &top)
            .map_err(|failure| WorktreeError::Failed(failure.reason()))?;
        // A record whose folder is gone holds nothing, so it is skipped.
        let is_inside_a_worktree = recorded
            .iter()
            .filter_map(|folder| folder.canonicalize().ok())
            .any(|folder| root.starts_with(folder));
        if root.starts_with(&top) || is_inside_a_worktree {
            return Err(WorktreeError::Invalid(DATA_FOLDER_INSIDE.to_owned()));
        }
        std::fs::create_dir_all(&self.root).map_err(|error| {
            WorktreeError::Failed(format!("the worktrees folder cannot be made: {error}"))
        })
    }

    /// The undo of a create deletes the branch it just made, whatever it
    /// points at. The answer names the branch when git did not confirm it
    /// gone.
    fn undo_branch(&self, worktree: &AgentWorktree) -> Option<String> {
        let runner = self.runner()?;
        let deleted = runner.write(
            &worktree.repository,
            &["branch", "-D", "--", &worktree.branch],
        );
        let Err(failure) = deleted else {
            return None;
        };
        // An undo after a timeout may find no branch at all.
        let is_gone = matches!(
            runner.read(
                &worktree.repository,
                &["rev-parse", "--verify", "--quiet", &worktree.branch_ref()],
            ),
            Err(GitFailure::Exit(_))
        );
        (!is_gone).then(|| {
            format!(
                "the branch {} was left behind: {}",
                worktree.branch,
                failure.reason()
            )
        })
    }

    /// Raises what an undo left behind as a storage alert: the answer
    /// already carries the failure that began the undo.
    fn report_leftovers(&self, worktree: &AgentWorktree, leftovers: &[String]) {
        if leftovers.is_empty() {
            return;
        }
        let Some(failures) = &self.failures else {
            return;
        };
        failures.report(&PortError::Storage {
            message: format!(
                "the undo of the worktree {} was not complete: {}",
                worktree.folder.display(),
                leftovers.join("; ")
            ),
            is_disk_failure: true,
        });
    }

    /// Writes each place whose links were not made to the server log; the
    /// create goes on without them (0021 D5).
    fn report_link_failures(&self, reasons: &[String]) {
        let Some(failures) = &self.failures else {
            return;
        };
        for reason in reasons {
            failures.report(&PortError::Storage {
                message: reason.clone(),
                is_disk_failure: false,
            });
        }
    }

    /// `worktree remove` with `force` times `--force`, after the check that
    /// the folder is the office's own and after the dependency links are
    /// gone.
    fn remove_record(&self, worktree: &AgentWorktree, force: usize) -> Result<(), WorktreeError> {
        if !self.is_owned(&worktree.folder) {
            return Err(WorktreeError::Invalid(format!(
                "the worktree folder is not in the office's worktrees folder: {}",
                worktree.folder.display()
            )));
        }
        let runner = self.require_runner()?;
        unlink_linked_dependencies(runner, worktree);
        // Without `--force` git runs its own status in the folder, which
        // would run the filters too.
        let filter_switches = if force == 0 && worktree.folder.exists() {
            linked_filter_switches(runner, worktree)?
        } else {
            Vec::new()
        };
        let mut arguments: Vec<&OsStr> = filter_switches.iter().map(OsStr::new).collect();
        // Git's own clean check follows `status.showUntrackedFiles`; with
        // `no` a plain remove would delete a new untracked file unasked.
        arguments.extend([
            OsStr::new("-c"),
            OsStr::new(SHOW_UNTRACKED_FILES),
            OsStr::new("worktree"),
            OsStr::new("remove"),
        ]);
        arguments.extend(std::iter::repeat_n(OsStr::new("--force"), force));
        arguments.extend([OsStr::new("--"), worktree.folder.as_os_str()]);
        // Deleting a large folder can take as long as checking it out.
        runner
            .checkout(&worktree.repository, &arguments)
            .map(|_| ())
            .map_err(|failure| WorktreeError::Failed(failure.reason()))
    }

    fn make(
        &self,
        runner: &GitRunner,
        info: &WorktreeInfo,
        request: &WorktreeCreate<'_>,
        base: &str,
        branch: &str,
    ) -> Result<CreatedWorktree, WorktreeError> {
        let folder = self.folder_for(&info.top, &request.agent_id.to_string());
        if folder.exists() {
            return Err(WorktreeError::Failed(format!(
                "the worktree folder already exists: {}",
                folder.display()
            )));
        }
        if let Some(parent) = folder.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                WorktreeError::Failed(format!("the worktree folder cannot be made: {error}"))
            })?;
        }
        let worktree = AgentWorktree {
            repository: info.top.clone(),
            base: base.to_owned(),
            branch: branch.to_owned(),
            folder,
        };
        let branched = runner.write(
            &info.top,
            &["branch", "--no-track", "--", branch, &base_ref(base)],
        );
        if let Err(failure) = branched {
            // A refused branch was never made, and the name may be the
            // user's own; only a branch step ended mid-way is undone.
            if failure == GitFailure::Timeout {
                self.report_leftovers(&worktree, self.undo_branch(&worktree).as_slice());
                return Err(create_failure(&failure));
            }
            // Another create may have taken the name since the info.
            return Err(taken_since(runner, &info.top, request.agent_name, branch)
                .unwrap_or_else(|| create_failure(&failure)));
        }
        let added = runner.checkout(
            &info.top,
            &[
                OsStr::new("worktree"),
                OsStr::new("add"),
                OsStr::new("--"),
                worktree.folder.as_os_str(),
                OsStr::new(branch),
            ],
        );
        if let Err(failure) = added {
            self.discard(&worktree);
            return Err(create_failure(&failure));
        }
        let working_folder = subfolder_of(&worktree.folder, &info.subfolder);
        if !is_inside(&working_folder, &worktree.folder) {
            self.discard(&worktree);
            return Err(WorktreeError::Invalid(format!(
                "the folder {} does not exist in {base}",
                info.subfolder.trim_end_matches('/')
            )));
        }
        if request.link_dependencies {
            let failures = link_dependencies(runner, &info.top, &worktree.folder);
            self.report_link_failures(&failures);
        }
        Ok(CreatedWorktree {
            worktree,
            working_folder,
        })
    }
}

impl WorktreeManager for GitWorktrees {
    fn info(&self, folder: &Path, agent_name: &str) -> Result<Option<WorktreeInfo>, WorktreeError> {
        let Some(runner) = self.runner() else {
            return Ok(None);
        };
        if !folder.is_dir() {
            return Ok(None);
        }
        let Ok(location) = runner.read(folder, &["rev-parse", "--show-toplevel", "--show-prefix"])
        else {
            return Ok(None);
        };
        let mut lines = location.lines();
        let (Some(top), subfolder) = (lines.next(), lines.next().unwrap_or_default()) else {
            return Ok(None);
        };
        // A folder inside a linked worktree belongs to the main one, which
        // outlives every linked worktree (another agent's included).
        let top = main_worktree(runner, Path::new(top)).unwrap_or_else(|| PathBuf::from(top));
        let Ok(branches) = branch_names(runner, &top) else {
            return Ok(None);
        };
        let Some((base, base_is_branch)) = checked_out_base(runner, folder, &branches) else {
            // A repository with no commit yet has no base to start from.
            return Ok(None);
        };
        let branch_name = free_branch_name(agent_name, &branches);
        check_ref_format(runner, folder, &branch_name)?;
        Ok(Some(WorktreeInfo {
            top,
            subfolder: subfolder.to_owned(),
            base,
            base_is_branch,
            branches,
            branch_name,
            // The handler that knows the settings decides (0021 FR-008).
            shares_dependencies: false,
        }))
    }

    fn create(&self, request: &WorktreeCreate<'_>) -> Result<CreatedWorktree, WorktreeError> {
        let runner = self.require_runner()?;
        let info = self
            .info(request.folder, request.agent_name)?
            .ok_or_else(|| WorktreeError::Invalid(NEEDS_REPOSITORY.to_owned()))?;
        self.ensure_outside(runner, &info.top)?;
        let base = chosen_base(&info, request.base)?;
        let branch = request.branch_name.unwrap_or(&info.branch_name);
        if !is_office_branch_name(branch) {
            return Err(WorktreeError::Invalid(BRANCH_NAME_SHAPE.to_owned()));
        }
        check_ref_format(runner, request.folder, branch)?;
        if info.branches.iter().any(|existing| existing == branch) {
            return Err(taken_branch(branch, &info.branch_name));
        }
        self.make(runner, &info, request, base, branch)
    }

    fn status(&self, worktree: &AgentWorktree) -> WorktreeStatus {
        let folder_exists = worktree.folder.is_dir();
        let unreachable = WorktreeStatus {
            folder_exists,
            reachable: false,
            changed_files: 0,
            status_hash: hex_hash(&[]),
            own_commits: None,
            base_exists: false,
            branch_commit: None,
        };
        let Some(runner) = self.runner() else {
            return unreachable;
        };
        if folder_exists && !is_linked_worktree(&worktree.folder, &worktree.repository) {
            // Without its `.git` link to the repository git would search
            // upward, or follow the link, and read some other repository.
            return unreachable;
        }
        let status = if folder_exists {
            let Ok(switches) = filter_switches(runner, &worktree.folder) else {
                return unreachable;
            };
            let arguments: Vec<&str> = switches
                .iter()
                .map(String::as_str)
                .chain(STATUS_ARGUMENTS)
                .collect();
            runner.read(&worktree.folder, &arguments)
        } else {
            // The record can still be cleared through the repository.
            runner
                .read(&worktree.repository, &["rev-parse", "--git-dir"])
                .map(|_| String::new())
        };
        let Ok(status) = status else {
            return unreachable;
        };
        let base = format!("{}^{{commit}}", worktree.base_ref());
        let base_exists = runner
            .read(
                &worktree.repository,
                &["rev-parse", "--verify", "--quiet", &base],
            )
            .is_ok();
        let branch_commit = runner
            .read(
                &worktree.repository,
                &[
                    "rev-parse",
                    "--verify",
                    "--quiet",
                    &format!("{}^{{commit}}", worktree.branch_ref()),
                ],
            )
            .ok()
            .map(|commit| commit.trim().to_owned())
            .filter(|commit| is_commit_id(commit));
        // Counted at that commit, so the delete can require it unchanged.
        let own_commits = branch_commit
            .as_deref()
            .filter(|_| base_exists)
            .and_then(|commit| {
                let range = format!("{}..{commit}", worktree.base_ref());
                runner
                    .read(&worktree.repository, &["rev-list", "--count", &range, "--"])
                    .ok()
            })
            .and_then(|count| count.trim().parse().ok());
        let changed = changed_paths(&status);
        WorktreeStatus {
            folder_exists,
            reachable: true,
            changed_files: changed.len(),
            status_hash: status_hash(&worktree.folder, &status, &changed),
            own_commits,
            base_exists,
            branch_commit,
        }
    }

    fn remove(&self, worktree: &AgentWorktree, force: bool) -> Result<(), WorktreeError> {
        self.remove_record(worktree, usize::from(force))
    }

    fn delete_branch(&self, worktree: &AgentWorktree, commit: &str) -> Result<(), WorktreeError> {
        let runner = self.require_runner()?;
        let records = runner
            .read(&worktree.repository, &["worktree", "list", "--porcelain"])
            .map_err(|failure| WorktreeError::Failed(failure.reason()))?;
        let checked_out = format!("{BRANCH_RECORD_PREFIX}{}", worktree.branch_ref());
        if records.lines().any(|line| line == checked_out) {
            return Err(WorktreeError::Failed(format!(
                "the branch {} is checked out in another folder",
                worktree.branch
            )));
        }
        // One step that deletes the ref only while it still points at the
        // counted commit, so a commit made since is never lost.
        runner
            .write(
                &worktree.repository,
                &["update-ref", "-d", &worktree.branch_ref(), commit],
            )
            .map(|_| ())
            .map_err(|failure| WorktreeError::Failed(failure.reason()))
    }

    fn discard(&self, worktree: &AgentWorktree) {
        // Each step is best effort: the undo removes what the create made,
        // in whatever state the failure left it, and then names what is
        // still there.
        // A `worktree add` ended by a timeout leaves its new worktree
        // locked; only a double force removes a locked one.
        let _ = self.remove_record(worktree, DISCARD_FORCE);
        if self.is_really_owned(&worktree.folder) {
            let _ = std::fs::remove_dir_all(&worktree.folder);
            // A folder git could not remove leaves a record whose folder is
            // now gone; the forced remove clears that one record. A record
            // still there keeps the branch, so the branch names it.
            let _ = self.remove_record(worktree, DISCARD_FORCE);
        }
        let mut leftovers = Vec::new();
        if worktree.folder.exists() {
            leftovers.push(format!(
                "the folder {} was left behind",
                worktree.folder.display()
            ));
        }
        leftovers.extend(self.undo_branch(worktree));
        self.report_leftovers(worktree, &leftovers);
    }
}

/// Removes the dependency links of a worktree still linked to its
/// repository; git would read another repository through a broken link
/// (0021 D7).
fn unlink_linked_dependencies(runner: &GitRunner, worktree: &AgentWorktree) {
    if is_linked_worktree(&worktree.folder, &worktree.repository) {
        unlink_dependencies(runner, &worktree.repository, &worktree.folder);
    }
}

/// The main worktree of the repository that holds `top`; `None` for a bare
/// repository, whose linked worktree is then the top.
fn main_worktree(runner: &GitRunner, top: &Path) -> Option<PathBuf> {
    let records = runner
        .read(top, &["worktree", "list", "--porcelain"])
        .ok()?;
    let mut lines = records.lines();
    let path = lines.next()?.strip_prefix(WORKTREE_RECORD_PREFIX)?;
    let is_bare = lines
        .take_while(|line| !line.is_empty())
        .any(|line| line == BARE_RECORD);
    (!is_bare).then(|| PathBuf::from(path))
}

/// Every worktree folder of the repository that holds `top`, as git
/// records them.
fn worktree_folders(runner: &GitRunner, top: &Path) -> Result<Vec<PathBuf>, GitFailure> {
    let records = runner.read(top, &["worktree", "list", "--porcelain"])?;
    Ok(records
        .lines()
        .filter_map(|line| line.strip_prefix(WORKTREE_RECORD_PREFIX))
        .map(PathBuf::from)
        .collect())
}

/// The repository's branch names.
fn branch_names(runner: &GitRunner, top: &Path) -> Result<Vec<String>, GitFailure> {
    let branch_list = runner.read(top, &["for-each-ref", "--format=%(refname)", "refs/heads/"])?;
    Ok(branch_list
        .lines()
        .filter_map(|line| line.strip_prefix(BRANCH_REF_PREFIX))
        .map(str::to_owned)
        .collect())
}

fn taken_branch(branch: &str, next_free: &str) -> WorktreeError {
    WorktreeError::Conflict(format!(
        "the branch {branch} exists; the next free name is {next_free}"
    ))
}

/// `Conflict` when `branch` exists now: another create took the name after
/// this one read the branches (D5).
fn taken_since(
    runner: &GitRunner,
    top: &Path,
    agent_name: &str,
    branch: &str,
) -> Option<WorktreeError> {
    let branches = branch_names(runner, top).ok()?;
    branches
        .iter()
        .any(|existing| existing == branch)
        .then(|| taken_branch(branch, &free_branch_name(agent_name, &branches)))
}

/// `filter_switches` of a worktree folder still linked to its repository;
/// the configuration of another repository says nothing about this one.
fn linked_filter_switches(
    runner: &GitRunner,
    worktree: &AgentWorktree,
) -> Result<Vec<String>, WorktreeError> {
    if !is_linked_worktree(&worktree.folder, &worktree.repository) {
        return Err(WorktreeError::Failed(format!(
            "the worktree folder is missing or broken: {}",
            worktree.folder.display()
        )));
    }
    filter_switches(runner, &worktree.folder).map_err(WorktreeError::Failed)
}

/// The branch checked out in `folder`, or its commit when detached. `None`
/// when the branch has no commit yet.
fn checked_out_base(
    runner: &GitRunner,
    folder: &Path,
    branches: &[String],
) -> Option<(String, bool)> {
    if let Ok(head) = runner.read(folder, &["symbolic-ref", "--quiet", "HEAD"]) {
        let branch = head.trim().strip_prefix(BRANCH_REF_PREFIX)?.to_owned();
        return branches.contains(&branch).then_some((branch, true));
    }
    let commit = runner
        .read(folder, &["rev-parse", "--verify", "HEAD^{commit}"])
        .ok()?;
    let commit = commit.trim().to_owned();
    is_commit_id(&commit).then_some((commit, false))
}

/// The client's base, which must be one the info listed (D3).
fn chosen_base<'a>(info: &'a WorktreeInfo, base: Option<&str>) -> Result<&'a str, WorktreeError> {
    let base = base.unwrap_or(&info.base);
    if !info.base_is_branch && base == info.base {
        return Ok(&info.base);
    }
    if is_commit_id(base) {
        // `base_ref` would pass such a name as a commit id.
        return Err(WorktreeError::Invalid(format!(
            "base is a branch named like a commit id; pick another base: {base}"
        )));
    }
    info.branches
        .iter()
        .find(|branch| *branch == base)
        .map(String::as_str)
        .ok_or_else(|| {
            WorktreeError::Invalid(format!("base is not a branch of the repository: {base}"))
        })
}

fn check_ref_format(runner: &GitRunner, folder: &Path, branch: &str) -> Result<(), WorktreeError> {
    runner
        .read(folder, &["check-ref-format", "--branch", branch])
        .map(|_| ())
        .map_err(|_| WorktreeError::Invalid(BRANCH_NAME_SHAPE.to_owned()))
}

fn create_failure(failure: &GitFailure) -> WorktreeError {
    WorktreeError::Failed(format!(
        "the worktree could not be created: {}",
        failure.reason()
    ))
}

/// The canonical form of `path`, built from its nearest existing ancestor,
/// so nothing has to be created to compare it.
fn canonical_existing(path: &Path) -> Option<PathBuf> {
    let mut missing = Vec::new();
    let mut existing = path;
    while !existing.exists() {
        missing.push(existing.file_name()?);
        existing = existing.parent()?;
    }
    let mut canonical = existing.canonicalize().ok()?;
    canonical.extend(missing.iter().rev());
    Some(canonical)
}

/// A folder that exists and, with every link followed, stays inside
/// `worktree`: a tracked symbolic link must not start the agent elsewhere.
fn is_inside(folder: &Path, worktree: &Path) -> bool {
    match (folder.canonicalize(), worktree.canonicalize()) {
        (Ok(folder), Ok(worktree)) => folder.is_dir() && folder.starts_with(worktree),
        _ => false,
    }
}

/// The worktree plus the `/`-separated subfolder git reported.
fn subfolder_of(worktree: &Path, subfolder: &str) -> PathBuf {
    subfolder
        .split('/')
        .filter(|part| !part.is_empty())
        .fold(worktree.to_path_buf(), |folder, part| folder.join(part))
}

/// The paths of `status --porcelain=v1 -z`: one per entry. A rename or a
/// copy carries its old path as one more field, which is skipped.
fn changed_paths(status: &str) -> Vec<&str> {
    const CODE_AND_SPACE: usize = 3;
    let mut fields = status.split('\0').filter(|field| !field.is_empty());
    let mut paths = Vec::new();
    while let Some(entry) = fields.next() {
        let Some(path) = entry.get(CODE_AND_SPACE..) else {
            continue;
        };
        if entry.starts_with(['R', 'C']) {
            fields.next();
        }
        paths.push(path);
    }
    paths
}

/// The status plus the size and change time of each changed file, so an
/// edit to a file that was already changed also changes the hash (0010 D6).
fn status_hash(folder: &Path, status: &str, changed: &[&str]) -> String {
    let mut input = status.as_bytes().to_vec();
    for path in changed {
        let Ok(metadata) = std::fs::symlink_metadata(folder.join(path)) else {
            continue;
        };
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |since| since.as_nanos());
        input.extend_from_slice(&metadata.len().to_le_bytes());
        input.extend_from_slice(&modified.to_le_bytes());
    }
    hex_hash(&input)
}

fn hex_hash(bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
#[path = "git_worktrees.tests.rs"]
mod tests;
