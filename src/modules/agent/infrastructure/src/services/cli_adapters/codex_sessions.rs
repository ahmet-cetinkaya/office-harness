// SPDX-License-Identifier: AGPL-3.0-or-later

//! Finds the id of the session that a started `codex` created, by reading
//! its session files. Nothing here writes: the CLI's folder is only listed
//! and read (FR-021).

use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use agent_application::common::interfaces::SessionTracking;
use agent_application::common::models::TrackedSession;
use serde_json::Value;

const CODEX_HOME_VARIABLE: &str = "CODEX_HOME";
const HOME_VARIABLES: [&str; 2] = ["HOME", "USERPROFILE"];
const CODEX_FOLDER: &str = ".codex";
const SESSIONS_FOLDER: &str = "sessions";
const SESSION_FILE_PREFIX: &str = "rollout-";
const SESSION_FILE_EXTENSION: &str = "jsonl";
/// Sessions live in `sessions/<year>/<month>/<day>/`.
const MAX_DEPTH: usize = 4;
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(200);
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);
/// After the first new file, a second one of the folder may still appear.
const DEFAULT_SETTLE: Duration = Duration::from_secs(1);
/// A session file names the session in its first line, which is short; a
/// file whose first line is longer is not one of ours, and the read of it
/// stops here.
const MAX_FIRST_LINE_BYTES: u64 = 1024 * 1024;
const UUID_GROUP_LENGTHS: [usize; 5] = [8, 4, 4, 4, 12];

const REASON_OVERLAP: &str = "another agent of this CLI started in the same folder at the same \
                              time, so the session cannot be told apart";
const REASON_SEVERAL: &str = "more than one new session of the CLI appeared in this folder";
const REASON_NONE: &str = "no new session of the CLI appeared in this folder in time";

/// How long, and how often, to look for the new session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttributionTiming {
    pub poll_interval: Duration,
    pub timeout: Duration,
    /// How long to keep looking after the first match, so that a second new
    /// session of the folder is seen before the first is trusted.
    pub settle: Duration,
}

impl Default for AttributionTiming {
    fn default() -> Self {
        Self {
            poll_interval: DEFAULT_POLL_INTERVAL,
            timeout: DEFAULT_TIMEOUT,
            settle: DEFAULT_SETTLE,
        }
    }
}

/// Where `codex` keeps its sessions for this environment: `CODEX_HOME`, or
/// the `.codex` folder of the home folder.
#[must_use]
pub fn sessions_folder(environment: &[(String, String)]) -> Option<PathBuf> {
    let variable = |name: &str| {
        environment
            .iter()
            .find(|(key, value)| key == name && !value.is_empty())
            .map(|(_, value)| PathBuf::from(value))
    };
    let home = variable(CODEX_HOME_VARIABLE).or_else(|| {
        HOME_VARIABLES
            .iter()
            .find_map(|name| variable(name))
            .map(|home| home.join(CODEX_FOLDER))
    })?;
    Some(home.join(SESSIONS_FOLDER))
}

/// The starts that look for a session right now, continues included: a
/// continued session may write a file too. Two starts in one folder that
/// overlap cannot tell their sessions apart, so both give up (research R3:
/// the adapter never guesses). A start that found nothing stays registered
/// for one more timeout, so a file that appears late is not taken by a start
/// that begins right after.
#[derive(Default)]
pub struct PendingStarts(Mutex<Vec<PendingStart>>);

struct PendingStart {
    folder: PathBuf,
    ambiguous: Arc<AtomicBool>,
}

/// Registered while a start looks for its session; leaving ends it.
struct PendingGuard {
    owner: Arc<PendingStarts>,
    ambiguous: Arc<AtomicBool>,
}

impl PendingStarts {
    /// The folder is compared canonicalized, as the match of a session file
    /// does, so a symlink or a `..` cannot hide an overlap.
    fn register(self: &Arc<Self>, folder: &Path) -> PendingGuard {
        let folder = folder
            .canonicalize()
            .unwrap_or_else(|_| folder.to_path_buf());
        let ambiguous = Arc::new(AtomicBool::new(false));
        let mut pending = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let overlaps = pending.iter().any(|other| other.folder == folder);
        if overlaps {
            ambiguous.store(true, Ordering::SeqCst);
            for other in pending.iter().filter(|other| other.folder == folder) {
                other.ambiguous.store(true, Ordering::SeqCst);
            }
        }
        pending.push(PendingStart {
            folder,
            ambiguous: Arc::clone(&ambiguous),
        });
        PendingGuard {
            owner: Arc::clone(self),
            ambiguous,
        }
    }
}

impl Drop for PendingGuard {
    fn drop(&mut self) {
        let mut pending = self.owner.0.lock().unwrap_or_else(PoisonError::into_inner);
        pending.retain(|other| !Arc::ptr_eq(&other.ambiguous, &self.ambiguous));
    }
}

/// One start of `codex`, from the list of session files before it to the
/// matching of the new one.
pub struct CodexTracking {
    sessions_folder: PathBuf,
    working_folder: PathBuf,
    /// Files that are listed before the start, or are rejected for good, and
    /// need no further look.
    known_files: HashSet<PathBuf>,
    timing: AttributionTiming,
    guard: PendingGuard,
}

impl CodexTracking {
    /// Takes the list of session files that exist now. Call it before the
    /// start, so the start's own file is the only new one.
    #[must_use]
    pub fn begin(
        sessions_folder: PathBuf,
        working_folder: &Path,
        timing: AttributionTiming,
        pending: &Arc<PendingStarts>,
    ) -> Self {
        let guard = pending.register(working_folder);
        let known_files = session_files(&sessions_folder).into_iter().collect();
        Self {
            sessions_folder,
            working_folder: working_folder.to_path_buf(),
            known_files,
            timing,
            guard,
        }
    }

    /// Looks until the session is matched, is ambiguous, or the time is up.
    /// A match is returned only after the settle time passed without a
    /// second new session of the folder.
    #[must_use]
    pub fn resolve(&mut self) -> TrackedSession {
        let deadline = Instant::now() + self.timing.timeout;
        let mut settles_at: Option<Instant> = None;
        loop {
            let found = self.new_sessions_of_the_folder();
            // Checked after the files: a start that overlapped before this
            // point has already marked this one.
            if self.guard.ambiguous.load(Ordering::SeqCst) {
                return TrackedSession::Unmatched(REASON_OVERLAP.to_owned());
            }
            match found.as_slice() {
                [] if Instant::now() < deadline => thread::sleep(self.timing.poll_interval),
                [] => return TrackedSession::Unmatched(REASON_NONE.to_owned()),
                [only] => {
                    let settled =
                        *settles_at.get_or_insert_with(|| Instant::now() + self.timing.settle);
                    if Instant::now() >= settled {
                        return TrackedSession::Found(only.clone());
                    }
                    thread::sleep(self.timing.poll_interval);
                }
                _ => return TrackedSession::Unmatched(REASON_SEVERAL.to_owned()),
            }
        }
    }

    fn new_sessions_of_the_folder(&mut self) -> Vec<String> {
        let unknown: Vec<PathBuf> = session_files(&self.sessions_folder)
            .into_iter()
            .filter(|file| !self.known_files.contains(file))
            .collect();
        let mut ids = Vec::new();
        for file in unknown {
            match read_session_file(&file) {
                SessionFile::Incomplete => {}
                SessionFile::Session(meta)
                    if same_folder(&meta.working_folder, &self.working_folder) =>
                {
                    ids.push(meta.id);
                }
                // Another folder, or not a session file: it will not change.
                SessionFile::Rejected | SessionFile::Session(_) => {
                    self.known_files.insert(file);
                }
            }
        }
        ids
    }
}

impl SessionTracking for CodexTracking {
    fn follow(mut self: Box<Self>, report: Box<dyn FnOnce(TrackedSession) + Send>) {
        thread::spawn(move || {
            let tracked = self.resolve();
            let found_nothing =
                matches!(&tracked, TrackedSession::Unmatched(reason) if reason == REASON_NONE);
            report(tracked);
            if found_nothing {
                thread::sleep(self.timing.timeout);
            }
        });
    }
}

struct SessionMeta {
    id: String,
    working_folder: PathBuf,
}

fn session_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_session_files(root, 0, &mut files);
    files
}

fn collect_session_files(folder: &Path, depth: usize, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        // `DirEntry::file_type` does not follow symlinks: a link, a FIFO, or
        // a device is neither descended into nor read.
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_dir() {
            if depth < MAX_DEPTH {
                collect_session_files(&path, depth + 1, files);
            }
        } else if kind.is_file() && is_session_file(&path) {
            files.push(path);
        }
    }
}

fn is_session_file(path: &Path) -> bool {
    let named_like_a_session = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(SESSION_FILE_PREFIX));
    let has_extension = path
        .extension()
        .is_some_and(|extension| extension == SESSION_FILE_EXTENSION);
    named_like_a_session && has_extension
}

/// What the first line of a file says about it.
enum SessionFile {
    /// The line is not complete yet: the file may be read again.
    Incomplete,
    /// Not a session file of the CLI, and it will not become one.
    Rejected,
    Session(SessionMeta),
}

/// The id and the folder in the first line of a session file. At most
/// `MAX_FIRST_LINE_BYTES` are read, and only a UUID is taken as an id.
fn read_session_file(file: &Path) -> SessionFile {
    let Ok(opened) = fs::File::open(file) else {
        return SessionFile::Incomplete;
    };
    let mut first_line = String::new();
    let mut reader = BufReader::new(opened.take(MAX_FIRST_LINE_BYTES));
    if reader.read_line(&mut first_line).is_err() {
        return SessionFile::Incomplete;
    }
    let is_complete = first_line.ends_with('\n');
    if !is_complete
        && u64::try_from(first_line.len()).is_ok_and(|read| read >= MAX_FIRST_LINE_BYTES)
    {
        return SessionFile::Rejected;
    }
    let Ok(line) = serde_json::from_str::<Value>(first_line.trim()) else {
        return SessionFile::Incomplete;
    };
    let meta = line.get("payload").unwrap_or(&line);
    let id = meta.get("id").and_then(Value::as_str);
    let folder = meta.get("cwd").and_then(Value::as_str);
    match (id, folder) {
        (Some(id), Some(folder)) if is_uuid(id) => SessionFile::Session(SessionMeta {
            id: id.to_owned(),
            working_folder: PathBuf::from(folder),
        }),
        _ if is_complete => SessionFile::Rejected,
        _ => SessionFile::Incomplete,
    }
}

/// The 8-4-4-4-12 hexadecimal shape of a session id.
fn is_uuid(text: &str) -> bool {
    let groups: Vec<&str> = text.split('-').collect();
    groups.len() == UUID_GROUP_LENGTHS.len()
        && groups
            .iter()
            .zip(UUID_GROUP_LENGTHS)
            .all(|(group, length)| {
                group.len() == length && group.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
}

fn same_folder(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

#[cfg(test)]
#[path = "codex_sessions.tests.rs"]
mod tests;
