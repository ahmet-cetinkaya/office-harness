// SPDX-License-Identifier: AGPL-3.0-or-later

//! The agent's own branch and worktree through the system git program
//! (0010 D1 to D6).

use std::ffi::OsStr;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, OnceLock};

use agent_application::common::interfaces::{
    CreatedWorktree, WorktreeCreate, WorktreeError, WorktreeInfo, WorktreeManager, WorktreeStatus,
};
use agent_application::common::worktree_link::is_linked_worktree;
use agent_domain::value_objects::{
    AgentWorktree, base_ref, free_branch_name, is_commit_id, is_office_branch_name,
};

use crate::services::git_runner::{GitFailure, GitRunner};
use crate::services::login_environment::{Environment, LoginEnvironment};
use crate::services::program_lookup::find_program;

const GIT_PROGRAM: &str = "git";
const BRANCH_REF_PREFIX: &str = "refs/heads/";
const WORKTREE_RECORD_PREFIX: &str = "worktree ";
const BARE_RECORD: &str = "bare";
const SHOW_UNTRACKED_FILES: &str = "status.showUntrackedFiles=normal";
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

/// Where the git runner comes from: built once, on first use, because the
/// login environment takes a login shell to read.
enum RunnerSource {
    Login {
        environment: Arc<LoginEnvironment>,
        server_environment: Environment,
    },
    Fixed(Option<GitRunner>),
}

/// `WorktreeManager` over the system git program. Worktrees live under
/// `root`, which is outside every repository it serves.
pub struct GitWorktrees {
    root: PathBuf,
    source: RunnerSource,
    runner: OnceLock<Option<GitRunner>>,
}

impl GitWorktrees {
    /// Finds git on the login environment's `PATH` on first use.
    #[must_use]
    pub fn new(
        root: PathBuf,
        environment: Arc<LoginEnvironment>,
        server_environment: Environment,
    ) -> Self {
        Self {
            root: absolute(root),
            source: RunnerSource::Login {
                environment,
                server_environment,
            },
            runner: OnceLock::new(),
        }
    }

    /// Uses `runner`, or no git at all for `None`; for tests.
    #[must_use]
    pub fn with_runner(root: PathBuf, runner: Option<GitRunner>) -> Self {
        Self {
            root: absolute(root),
            source: RunnerSource::Fixed(runner),
            runner: OnceLock::new(),
        }
    }

    fn runner(&self) -> Option<&GitRunner> {
        self.runner
            .get_or_init(|| {
                let runner = match &self.source {
                    RunnerSource::Fixed(runner) => runner.clone(),
                    RunnerSource::Login {
                        environment,
                        server_environment,
                    } => login_runner(&self.root, environment, server_environment),
                }?;
                runner.is_supported().then_some(runner)
            })
            .as_ref()
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

    /// Refuses a worktrees folder inside the repository before anything is
    /// written there (A1, PB-2).
    fn ensure_outside(&self, top: &Path) -> Result<(), WorktreeError> {
        let (Some(root), Ok(top)) = (canonical_existing(&self.root), top.canonicalize()) else {
            return Err(WorktreeError::Failed(
                "the worktrees folder cannot be read".to_owned(),
            ));
        };
        if root.starts_with(&top) {
            return Err(WorktreeError::Invalid(DATA_FOLDER_INSIDE.to_owned()));
        }
        std::fs::create_dir_all(&self.root).map_err(|error| {
            WorktreeError::Failed(format!("the worktrees folder cannot be made: {error}"))
        })
    }

    /// `worktree remove` with `force` times `--force`, after the check that
    /// the folder is the office's own.
    fn remove_record(&self, worktree: &AgentWorktree, force: usize) -> Result<(), WorktreeError> {
        if !self.is_owned(&worktree.folder) {
            return Err(WorktreeError::Invalid(format!(
                "the worktree folder is not in the office's worktrees folder: {}",
                worktree.folder.display()
            )));
        }
        let runner = self.require_runner()?;
        // Git's own clean check follows `status.showUntrackedFiles`; with
        // `no` a plain remove would delete a new untracked file unasked.
        let mut arguments = vec![
            OsStr::new("-c"),
            OsStr::new(SHOW_UNTRACKED_FILES),
            OsStr::new("worktree"),
            OsStr::new("remove"),
        ];
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
                let _ = self.delete_branch(&worktree);
            }
            return Err(create_failure(&failure));
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
        let Ok(branch_list) = runner.read(
            &top,
            &["for-each-ref", "--format=%(refname)", "refs/heads/"],
        ) else {
            return Ok(None);
        };
        let branches: Vec<String> = branch_list
            .lines()
            .filter_map(|line| line.strip_prefix(BRANCH_REF_PREFIX))
            .map(str::to_owned)
            .collect();
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
        }))
    }

    fn create(&self, request: &WorktreeCreate<'_>) -> Result<CreatedWorktree, WorktreeError> {
        let runner = self.require_runner()?;
        let info = self
            .info(request.folder, request.agent_name)?
            .ok_or_else(|| WorktreeError::Invalid(NEEDS_REPOSITORY.to_owned()))?;
        self.ensure_outside(&info.top)?;
        let base = chosen_base(&info, request.base)?;
        let branch = request.branch_name.unwrap_or(&info.branch_name);
        if !is_office_branch_name(branch) {
            return Err(WorktreeError::Invalid(BRANCH_NAME_SHAPE.to_owned()));
        }
        check_ref_format(runner, request.folder, branch)?;
        if info.branches.iter().any(|existing| existing == branch) {
            return Err(WorktreeError::Conflict(format!(
                "the branch {branch} exists; the next free name is {}",
                info.branch_name
            )));
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
        };
        let Some(runner) = self.runner() else {
            return unreachable;
        };
        if folder_exists && !is_linked_worktree(&worktree.folder) {
            // Without its `.git` link git would search upward and read some
            // other repository.
            return unreachable;
        }
        let status = if folder_exists {
            runner.read(
                &worktree.folder,
                &[
                    "status",
                    "--porcelain=v1",
                    "--untracked-files=all",
                    "--ignore-submodules=none",
                    "-z",
                ],
            )
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
        let own_commits = base_exists
            .then(|| {
                let range = format!("{}..{}", worktree.base_ref(), worktree.branch_ref());
                runner.read(&worktree.repository, &["rev-list", "--count", &range, "--"])
            })
            .and_then(Result::ok)
            .and_then(|count| count.trim().parse().ok());
        let changed = changed_paths(&status);
        WorktreeStatus {
            folder_exists,
            reachable: true,
            changed_files: changed.len(),
            status_hash: status_hash(&worktree.folder, &status, &changed),
            own_commits,
            base_exists,
        }
    }

    fn remove(&self, worktree: &AgentWorktree, force: bool) -> Result<(), WorktreeError> {
        self.remove_record(worktree, usize::from(force))
    }

    fn delete_branch(&self, worktree: &AgentWorktree) -> Result<(), WorktreeError> {
        let runner = self.require_runner()?;
        runner
            .write(
                &worktree.repository,
                &["branch", "-D", "--", &worktree.branch],
            )
            .map(|_| ())
            .map_err(|failure| WorktreeError::Failed(failure.reason()))
    }

    fn discard(&self, worktree: &AgentWorktree) {
        // Each step is best effort: the undo removes what the create made,
        // in whatever state the failure left it.
        // A `worktree add` ended by a timeout leaves its new worktree
        // locked; only a double force removes a locked one.
        let _ = self.remove_record(worktree, DISCARD_FORCE);
        if self.is_really_owned(&worktree.folder) {
            let _ = std::fs::remove_dir_all(&worktree.folder);
            // A folder git could not remove leaves a record whose folder is
            // now gone; the forced remove clears that one record.
            let _ = self.remove_record(worktree, DISCARD_FORCE);
        }
        let _ = self.delete_branch(worktree);
    }
}

fn login_runner(
    root: &Path,
    environment: &LoginEnvironment,
    server_environment: &Environment,
) -> Option<GitRunner> {
    let login = environment.resolve(server_environment).variables;
    let program = find_program(GIT_PROGRAM, &login, root)?;
    Some(GitRunner::new(program, &login))
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

fn absolute(root: PathBuf) -> PathBuf {
    // A relative data folder would make the hooks path relative to each
    // worktree and put new worktrees inside the repository.
    std::path::absolute(&root).unwrap_or(root)
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
