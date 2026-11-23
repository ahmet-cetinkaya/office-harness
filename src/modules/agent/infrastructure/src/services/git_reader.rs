// SPDX-License-Identifier: AGPL-3.0-or-later

//! The git side of the file view (0016 D6, D7): changes against HEAD, one
//! file's diff, the history, and one commit. Every run is a read run of the
//! office's git runner with every external program switched off.

use std::path::Path;
use std::sync::Arc;

use agent_application::common::interfaces::{DiffRequest, FileViewError};
use agent_application::common::models::{
    ChangeKind, ChangeList, ChangedFile, CommitDetail, CommitPage, CommitSummary, ContentState,
    DiffState, FileDiff,
};
use agent_domain::services::{
    COMMITS_PER_PAGE, MAX_CHANGED_FILES, MAX_CONTENT_BYTES, MAX_MESSAGE_BYTES,
    MAX_STATUS_OUTPUT_BYTES, MAX_SUBJECT_CHARS, RelativePath, is_secret_looking,
};
use kernel::Timestamp;

use crate::services::folder_reader::{now, read_file};
use crate::services::git_parsing::{
    ChangeRecord, DiffBody, added_file_diff, diff_body, parse_log, parse_name_status, parse_status,
};
use crate::services::git_runner::{
    GitFailure, GitRunner, LazyGitRunner, STDOUT_LIMIT_BYTES, filter_switches,
};

const GIT_MISSING: &str = "git is not available";
const NOT_A_REPOSITORY: &str = "not a git repository";
const STATUS_TOO_LARGE: &str = "the folder has more changes than the view can list";
const MESSAGE_TOO_LARGE: &str = "the commit message is too large";
const NOT_A_COMMIT: &str = "commit is not a commit of this repository";
const NO_REPOSITORY: &str = "the working folder is not in a git repository";
const HEAD: &str = "HEAD";
const FIELD_SEPARATOR: char = '\u{1f}';
const COMMIT_FIELDS: usize = 6;
const MILLIS_PER_SECOND: i64 = 1_000;
/// Global options of every run: pathspecs are plain paths, and a
/// configured signature check never starts a program.
const SAFE_OPTIONS: [&str; 3] = ["--literal-pathspecs", "-c", "log.showSignature=false"];
/// A submodule's own work tree is never read: git would run there with the
/// submodule's configuration, whose filters the switches do not cover.
const IGNORE_SUBMODULE_CHANGES: &str = "--ignore-submodules=dirty";
/// The words of a diff that never starts a program or reads a filter.
const DIFF_OPTIONS: [&str; 6] = [
    "--no-ext-diff",
    "--no-textconv",
    "--no-color",
    "-M",
    "--relative",
    IGNORE_SUBMODULE_CHANGES,
];
const STATUS_OPTIONS: [&str; 5] = [
    "status",
    "--porcelain=v2",
    "-z",
    "--untracked-files=all",
    IGNORE_SUBMODULE_CHANGES,
];
const LOG_FORMAT: &str = "--format=%H%x1f%h%x1f%an%x1f%at%x1f%s%x1e";
const COMMIT_FORMAT: &str = "--format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%B";

/// Reads one folder's git repository, with git found on first use.
pub struct GitReader {
    runner: Arc<LazyGitRunner>,
}

impl GitReader {
    #[must_use]
    pub fn new(runner: Arc<LazyGitRunner>) -> Self {
        Self { runner }
    }

    /// The files of `root` that differ from HEAD, at most 1,000.
    ///
    /// # Errors
    /// `Failed` when git is missing, does not answer, or lists too much.
    pub fn changes(&self, root: &Path) -> Result<ChangeList, FileViewError> {
        let Some(repository) = self.open(root)? else {
            return Ok(ChangeList {
                repository: false,
                files: Vec::new(),
                more: 0,
                read_at: now(),
            });
        };
        let output = repository
            .run_bytes(&status_arguments(&["--", "."]), MAX_STATUS_OUTPUT_BYTES)
            .map_err(|failure| failed_or(failure, &GitFailure::OutputTooLarge, STATUS_TOO_LARGE))?;
        let records = parse_status(&output)
            .into_iter()
            .filter_map(|record| repository.within_prefix(record));
        let (files, more) = listed_files(records);
        Ok(ChangeList {
            repository: true,
            files,
            more,
            read_at: now(),
        })
    }

    /// The change of one path, against HEAD or against a commit's first
    /// parent.
    ///
    /// # Errors
    /// `Invalid` for a commit that is not one of the repository, `Failed`
    /// when git is missing or does not answer.
    pub fn diff(&self, root: &Path, request: &DiffRequest) -> Result<FileDiff, FileViewError> {
        let repository = self
            .open(root)?
            .ok_or_else(|| FileViewError::Failed(NO_REPOSITORY.to_owned()))?;
        match &request.commit {
            Some(commit) => repository.commit_diff(request, commit),
            None => repository.working_diff(request),
        }
    }

    /// One page of the commits of HEAD that change a path inside `root`.
    ///
    /// # Errors
    /// `Failed` when git is missing or does not answer.
    pub fn history(&self, root: &Path, skip: u32) -> Result<CommitPage, FileViewError> {
        let Some(repository) = self.open(root)? else {
            return Ok(no_commits(false));
        };
        if !repository.has_head()? {
            return Ok(no_commits(true));
        }
        repository.history(skip)
    }

    /// One commit with its message and its files inside `root`.
    ///
    /// # Errors
    /// `Invalid` for a commit that is not one of the repository, `Failed`
    /// when git is missing or does not answer.
    pub fn commit(&self, root: &Path, commit: &str) -> Result<CommitDetail, FileViewError> {
        let repository = self
            .open(root)?
            .ok_or_else(|| FileViewError::Failed(NO_REPOSITORY.to_owned()))?;
        repository.commit(commit)
    }

    fn open<'a>(&'a self, root: &'a Path) -> Result<Option<Repository<'a>>, FileViewError> {
        let runner = self
            .runner
            .get()
            .ok_or_else(|| FileViewError::Failed(GIT_MISSING.to_owned()))?;
        Repository::open(runner, root)
    }
}

/// The repository that holds the root, and the arguments every run adds.
struct Repository<'a> {
    runner: &'a GitRunner,
    root: &'a Path,
    /// The root's path below the top of the work tree, `sub/` or empty.
    prefix: String,
    /// Each filter driver of the configuration switched off.
    filter_switches: Vec<String>,
}

impl<'a> Repository<'a> {
    fn open(runner: &'a GitRunner, root: &'a Path) -> Result<Option<Self>, FileViewError> {
        let prefix = match runner.read(root, &["rev-parse", "--show-prefix"]) {
            Ok(prefix) => prefix.trim_end().to_owned(),
            Err(GitFailure::Exit(reason)) if reason.contains(NOT_A_REPOSITORY) => return Ok(None),
            Err(failure) => return Err(failure.into()),
        };
        let filter_switches = filter_switches(runner, root).map_err(FileViewError::Failed)?;
        Ok(Some(Self {
            runner,
            root,
            prefix,
            filter_switches,
        }))
    }

    fn run_bytes(&self, arguments: &[&str], stdout_limit: usize) -> Result<Vec<u8>, GitFailure> {
        let all: Vec<&str> = SAFE_OPTIONS
            .into_iter()
            .chain(self.filter_switches.iter().map(String::as_str))
            .chain(arguments.iter().copied())
            .collect();
        self.runner.read_bytes(self.root, &all, stdout_limit)
    }

    fn run(&self, arguments: &[&str]) -> Result<String, FileViewError> {
        let output = self
            .run_bytes(arguments, STDOUT_LIMIT_BYTES)
            .map_err(FileViewError::from)?;
        Ok(String::from_utf8_lossy(&output).into_owned())
    }

    /// The record with its paths relative to the root; `None` for a path
    /// outside it. A rename whose old path is outside loses it.
    fn within_prefix(&self, record: ChangeRecord) -> Option<ChangeRecord> {
        Some(ChangeRecord {
            path: record.path.strip_prefix(&self.prefix)?.to_owned(),
            old_path: record
                .old_path
                .and_then(|old| old.strip_prefix(&self.prefix).map(str::to_owned)),
            kind: record.kind,
        })
    }

    fn has_head(&self) -> Result<bool, FileViewError> {
        let head_commit = format!("{HEAD}^{{commit}}");
        match self.run_bytes(
            &["rev-parse", "--verify", "--quiet", &head_commit],
            STDOUT_LIMIT_BYTES,
        ) {
            Ok(_) => Ok(true),
            Err(GitFailure::Exit(_)) => Ok(false),
            Err(failure) => Err(failure.into()),
        }
    }

    /// The id of the empty tree, which depends on the hash of the
    /// repository: what a commit without a parent is compared with.
    fn empty_tree(&self) -> Result<String, FileViewError> {
        Ok(self
            .run(&["hash-object", "-t", "tree", "--stdin"])?
            .trim()
            .to_owned())
    }

    fn working_diff(&self, request: &DiffRequest) -> Result<FileDiff, FileViewError> {
        if self.is_untracked(&request.path)? {
            return untracked_diff(self.root, &request.path);
        }
        let base = if self.has_head()? {
            HEAD.to_owned()
        } else {
            self.empty_tree()?
        };
        self.diff_against(&[&base], request)
    }

    fn is_untracked(&self, path: &RelativePath) -> Result<bool, FileViewError> {
        let path_text = path.to_text();
        let output = self
            .run_bytes(
                &status_arguments(&["--", &path_text]),
                MAX_STATUS_OUTPUT_BYTES,
            )
            .map_err(|failure| failed_or(failure, &GitFailure::OutputTooLarge, STATUS_TOO_LARGE))?;
        let expected = format!("{}{path_text}", self.prefix);
        Ok(parse_status(&output)
            .iter()
            .any(|record| record.kind == ChangeKind::Untracked && record.path == expected))
    }

    fn commit_diff(&self, request: &DiffRequest, commit: &str) -> Result<FileDiff, FileViewError> {
        self.verify_commit(commit)?;
        let parent = self.first_parent_or_empty_tree(commit)?;
        self.diff_against(&[&parent, commit], request)
    }

    fn diff_against(
        &self,
        revisions: &[&str],
        request: &DiffRequest,
    ) -> Result<FileDiff, FileViewError> {
        let path = request.path.to_text();
        let old_path = request.old_path.as_ref().map(RelativePath::to_text);
        let mut arguments = vec!["diff"];
        arguments.extend(DIFF_OPTIONS);
        arguments.extend(revisions);
        arguments.push("--");
        arguments.push(&path);
        arguments.extend(old_path.as_deref());
        let output = match self.run_bytes(&arguments, STDOUT_LIMIT_BYTES) {
            Ok(output) => output,
            Err(GitFailure::OutputTooLarge) => return Ok(file_diff(&path, DiffState::TooLarge)),
            Err(failure) => return Err(failure.into()),
        };
        Ok(match diff_body(&output) {
            DiffBody::Empty => file_diff(&path, DiffState::None),
            DiffBody::Binary => file_diff(&path, DiffState::Binary),
            DiffBody::Text(start) => text_diff(&path, &output[start..]),
        })
    }

    fn history(&self, skip: u32) -> Result<CommitPage, FileViewError> {
        let skip = format!("--skip={skip}");
        let page_and_one = (COMMITS_PER_PAGE + 1).to_string();
        let output = self
            .run_bytes(
                &[
                    "log",
                    "--no-show-signature",
                    "--no-decorate",
                    "--no-color",
                    LOG_FORMAT,
                    &skip,
                    "-n",
                    &page_and_one,
                    HEAD,
                    "--",
                    ".",
                ],
                STDOUT_LIMIT_BYTES,
            )
            .map_err(FileViewError::from)?;
        let mut records = parse_log(&output);
        let more = records.len() > COMMITS_PER_PAGE;
        records.truncate(COMMITS_PER_PAGE);
        Ok(CommitPage {
            repository: true,
            commits: records
                .into_iter()
                .map(|record| CommitSummary {
                    id: record.id,
                    short_id: record.short_id,
                    subject: record.subject.chars().take(MAX_SUBJECT_CHARS).collect(),
                    author: record.author,
                    time: seconds_to_time(record.author_seconds),
                })
                .collect(),
            more,
        })
    }

    fn commit(&self, commit: &str) -> Result<CommitDetail, FileViewError> {
        self.verify_commit(commit)?;
        let output = self
            .run_bytes(
                &["show", "-s", "--no-show-signature", COMMIT_FORMAT, commit],
                STDOUT_LIMIT_BYTES,
            )
            .map_err(|failure| {
                failed_or(failure, &GitFailure::OutputTooLarge, MESSAGE_TOO_LARGE)
            })?;
        let header = CommitHeader::parse(&String::from_utf8_lossy(&output));
        let base = match header.parents.first() {
            Some(parent) => parent.clone(),
            None => self.empty_tree()?,
        };
        let (files, more) = self.commit_files(&base, commit)?;
        Ok(CommitDetail {
            id: header.id,
            parents: header.parents,
            author: header.author,
            email: header.email,
            time: seconds_to_time(header.author_seconds),
            message: header.message,
            files,
            more,
        })
    }

    fn commit_files(
        &self,
        base: &str,
        commit: &str,
    ) -> Result<(Vec<ChangedFile>, usize), FileViewError> {
        let mut arguments = vec!["diff-tree", "-r", "-z", "--name-status"];
        arguments.extend(DIFF_OPTIONS);
        arguments.extend([base, commit, "--", "."]);
        let output = self
            .run_bytes(&arguments, MAX_STATUS_OUTPUT_BYTES)
            .map_err(|failure| failed_or(failure, &GitFailure::OutputTooLarge, STATUS_TOO_LARGE))?;
        Ok(listed_files(parse_name_status(&output)))
    }

    fn verify_commit(&self, commit: &str) -> Result<(), FileViewError> {
        let revision = format!("{commit}^{{commit}}");
        match self.run_bytes(
            &["rev-parse", "--verify", "--quiet", &revision],
            STDOUT_LIMIT_BYTES,
        ) {
            Ok(_) => Ok(()),
            Err(GitFailure::Exit(_)) => Err(FileViewError::Invalid(NOT_A_COMMIT.to_owned())),
            Err(failure) => Err(failure.into()),
        }
    }

    fn first_parent_or_empty_tree(&self, commit: &str) -> Result<String, FileViewError> {
        let parents = self.run(&["show", "-s", "--no-show-signature", "--format=%P", commit])?;
        match parents.split_whitespace().next() {
            Some(parent) => Ok(parent.to_owned()),
            None => self.empty_tree(),
        }
    }
}

/// The fields of `show --format=COMMIT_FORMAT`.
struct CommitHeader {
    id: String,
    parents: Vec<String>,
    author: String,
    email: String,
    author_seconds: i64,
    message: String,
}

impl CommitHeader {
    fn parse(output: &str) -> Self {
        let mut fields = output.splitn(COMMIT_FIELDS, FIELD_SEPARATOR);
        let mut next_field = || fields.next().unwrap_or_default();
        Self {
            id: next_field().to_owned(),
            parents: next_field().split_whitespace().map(str::to_owned).collect(),
            author: next_field().to_owned(),
            email: next_field().to_owned(),
            author_seconds: next_field().parse().unwrap_or_default(),
            message: limited_message(next_field()),
        }
    }
}

/// `status` with the options of every status run, then `extra`.
fn status_arguments<'a>(extra: &[&'a str]) -> Vec<&'a str> {
    let mut arguments = STATUS_OPTIONS.to_vec();
    arguments.extend(extra);
    arguments
}

/// The diff of a file git does not track, built from the file's bytes so
/// git never reads it.
fn untracked_diff(root: &Path, path: &RelativePath) -> Result<FileDiff, FileViewError> {
    let content = read_file(root, path, true)?;
    let name = path.to_text();
    Ok(match (content.state, content.bytes) {
        (ContentState::Text, Some(bytes)) => text_diff(&name, &added_file_diff(&bytes)),
        (ContentState::TooLarge, _) => FileDiff {
            size: Some(content.size),
            ..file_diff(&name, DiffState::TooLarge)
        },
        (ContentState::Secret, _) => file_diff(&name, DiffState::Secret),
        (ContentState::Binary | ContentState::Text, _) => file_diff(&name, DiffState::Binary),
    })
}

/// A diff of change text: none when empty, too large over the size limit.
fn text_diff(path: &str, body: &[u8]) -> FileDiff {
    if body.is_empty() {
        return file_diff(path, DiffState::None);
    }
    let size = u64::try_from(body.len()).ok();
    if body.len() > MAX_CONTENT_BYTES {
        return FileDiff {
            size,
            ..file_diff(path, DiffState::TooLarge)
        };
    }
    FileDiff {
        size,
        bytes: Some(body.to_vec()),
        ..file_diff(path, DiffState::Diff)
    }
}

fn file_diff(path: &str, state: DiffState) -> FileDiff {
    FileDiff {
        path: path.to_owned(),
        state,
        size: None,
        bytes: None,
    }
}

/// The changed files sorted by path, at most 1,000, and the count left out.
fn listed_files(records: impl IntoIterator<Item = ChangeRecord>) -> (Vec<ChangedFile>, usize) {
    let mut files: Vec<ChangedFile> = records.into_iter().map(changed_file).collect();
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let more = files.len().saturating_sub(MAX_CHANGED_FILES);
    files.truncate(MAX_CHANGED_FILES);
    (files, more)
}

/// A rename with a secret-looking old or new name is secret (FR-004).
fn changed_file(record: ChangeRecord) -> ChangedFile {
    let is_secret = |path: &str| path.rsplit('/').next().is_some_and(is_secret_looking);
    ChangedFile {
        secret: is_secret(&record.path) || record.old_path.as_deref().is_some_and(is_secret),
        path: record.path,
        kind: record.kind,
        old_path: record.old_path,
    }
}

fn no_commits(repository: bool) -> CommitPage {
    CommitPage {
        repository,
        commits: Vec::new(),
        more: false,
    }
}

fn seconds_to_time(seconds: i64) -> Timestamp {
    Timestamp::from_unix_millis(seconds.saturating_mul(MILLIS_PER_SECOND))
}

/// The message without its final line break, at most 64 KiB, cut at a
/// character.
fn limited_message(message: &str) -> String {
    let message = message.trim_end();
    let end = (0..=MAX_MESSAGE_BYTES.min(message.len()))
        .rev()
        .find(|end| message.is_char_boundary(*end))
        .unwrap_or_default();
    message[..end].to_owned()
}

impl From<GitFailure> for FileViewError {
    fn from(failure: GitFailure) -> Self {
        Self::Failed(failure.reason())
    }
}

/// `failed`, with `message` for the one failure `special`.
fn failed_or(failure: GitFailure, special: &GitFailure, message: &str) -> FileViewError {
    if &failure == special {
        return FileViewError::Failed(message.to_owned());
    }
    failure.into()
}

#[cfg(test)]
#[path = "git_reader.tests.rs"]
mod tests;
