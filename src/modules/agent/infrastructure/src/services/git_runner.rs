// SPDX-License-Identifier: AGPL-3.0-or-later

//! Runs the system `git` program for the worktree service (0010 D1): no
//! shell, a null input, a clean environment, a timeout that ends the whole
//! process tree, and bounded output.

use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::services::login_environment::Environment;

/// `core.hooksPath` for every run: a device, never a folder that anyone can
/// fill, so git finds no hook file there.
#[cfg(windows)]
pub const NO_HOOKS_PATH: &str = "core.hooksPath=NUL";
#[cfg(not(windows))]
pub const NO_HOOKS_PATH: &str = "core.hooksPath=/dev/null";
/// Newer git also runs hooks named in its configuration
/// (`hook.<name>.command`), which `core.hooksPath` does not turn off. These
/// switch off every hook of the events the office's commands can fire.
pub const DISABLED_HOOK_EVENTS: [&str; 4] = [
    "hook.post-checkout.enabled=false",
    "hook.reference-transaction.enabled=false",
    "hook.post-index-change.enabled=false",
    "hook.pre-auto-gc.enabled=false",
];

/// Read commands: status, rev-parse, rev-list, for-each-ref.
pub const READ_TIMEOUT: Duration = Duration::from_secs(10);
/// `worktree add` checks out every file and runs checkout filters.
pub const CHECKOUT_TIMEOUT: Duration = Duration::from_secs(120);
const STDOUT_LIMIT_BYTES: usize = 1024 * 1024;
const STDERR_LIMIT_BYTES: usize = 64 * 1024;
const REASON_MAX_CHARS: usize = 500;
const READ_CHUNK_BYTES: usize = 8 * 1024;
const EXIT_POLL: Duration = Duration::from_millis(10);
/// How long a pipe may stay open after git exited: the reader needs only
/// to take what git already wrote.
const PIPE_DRAIN: Duration = Duration::from_millis(500);
const MINIMUM_VERSION: (u32, u32) = (2, 22);
const GIT_VARIABLE_PREFIX: &str = "GIT_";
const ASKPASS_VARIABLES: [&str; 2] = ["SSH_ASKPASS", "SUDO_ASKPASS"];
/// Pinned so git's messages parse and read the same on every machine, and
/// so git never waits for a credential prompt.
const FIXED_VARIABLES: [(&str, &str); 3] = [
    ("LC_ALL", "C"),
    ("GIT_TERMINAL_PROMPT", "0"),
    ("GCM_INTERACTIVE", "never"),
];
/// Read commands do not refresh the index, so a status never takes a lock
/// that the user's own git would then find.
const NO_OPTIONAL_LOCKS: (&str, &str) = ("GIT_OPTIONAL_LOCKS", "0");
/// Read commands never fetch a missing object: in a partial clone that
/// fetch runs the transport the repository's configuration names.
const NO_LAZY_FETCH: (&str, &str) = ("GIT_NO_LAZY_FETCH", "1");

/// Why a git run failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitFailure {
    /// The program could not start.
    Start(String),
    /// The run took longer than its timeout and was ended.
    Timeout,
    /// More output than the limit.
    OutputTooLarge,
    /// git ended with an error; the text is its cleaned reason.
    Exit(String),
}

impl GitFailure {
    /// The text a user reads.
    #[must_use]
    pub fn reason(&self) -> String {
        match self {
            Self::Start(reason) => format!("git could not start: {reason}"),
            Self::Timeout => "git did not finish in time".to_owned(),
            Self::OutputTooLarge => "git answered with more output than expected".to_owned(),
            Self::Exit(reason) => reason.clone(),
        }
    }
}

/// The git program and the environment it runs with.
#[derive(Debug, Clone)]
pub struct GitRunner {
    program: PathBuf,
    environment: Environment,
    read_timeout: Duration,
    checkout_timeout: Duration,
}

impl GitRunner {
    /// A runner for `program`. `login_environment` is the user's environment;
    /// every `GIT_*` and askpass variable is dropped from it, so nothing in it
    /// can point git at another repository.
    #[must_use]
    pub fn new(program: PathBuf, login_environment: &Environment) -> Self {
        Self {
            program,
            environment: child_environment(login_environment),
            read_timeout: READ_TIMEOUT,
            checkout_timeout: CHECKOUT_TIMEOUT,
        }
    }

    /// Shorter timeouts, for tests that make git hang.
    #[must_use]
    pub fn with_timeouts(mut self, read: Duration, checkout: Duration) -> Self {
        self.read_timeout = read;
        self.checkout_timeout = checkout;
        self
    }

    /// True when the program answers `git version` with 2.22 or newer.
    #[must_use]
    pub fn is_supported(&self) -> bool {
        self.read(Path::new("."), &["version"])
            .ok()
            .and_then(|output| parse_version(&output))
            .is_some_and(|version| version >= MINIMUM_VERSION)
    }

    /// Runs a command that changes nothing, in `folder`, and returns its
    /// standard output.
    ///
    /// # Errors
    /// See `GitFailure`.
    pub fn read<A: AsRef<OsStr>>(
        &self,
        folder: &Path,
        arguments: &[A],
    ) -> Result<String, GitFailure> {
        self.run(folder, arguments, self.read_timeout, true)
    }

    /// Runs a command that changes the repository's git data.
    ///
    /// # Errors
    /// See `GitFailure`.
    pub fn write<A: AsRef<OsStr>>(
        &self,
        folder: &Path,
        arguments: &[A],
    ) -> Result<String, GitFailure> {
        self.run(folder, arguments, self.read_timeout, false)
    }

    /// Runs `worktree add` or `worktree remove`, which may take long on a
    /// large repository.
    ///
    /// # Errors
    /// See `GitFailure`.
    pub fn checkout<A: AsRef<OsStr>>(
        &self,
        folder: &Path,
        arguments: &[A],
    ) -> Result<String, GitFailure> {
        self.run(folder, arguments, self.checkout_timeout, false)
    }

    fn run<A: AsRef<OsStr>>(
        &self,
        folder: &Path,
        arguments: &[A],
        timeout: Duration,
        is_read: bool,
    ) -> Result<String, GitFailure> {
        let mut command = self.command(folder, is_read);
        command.args(arguments);
        let deadline = Instant::now() + timeout;
        let mut child = command
            .spawn()
            .map_err(|error| GitFailure::Start(error.to_string()))?;
        let stdout = child
            .stdout
            .take()
            .map(|pipe| bounded_reader(pipe, STDOUT_LIMIT_BYTES));
        let stderr = child
            .stderr
            .take()
            .map(|pipe| bounded_reader(pipe, STDERR_LIMIT_BYTES));
        let Some(status) = wait_until(&mut child, deadline) else {
            end_tree(&mut child);
            return Err(GitFailure::Timeout);
        };
        let (Some((stdout, stdout_overflow)), stderr) =
            exited_output(&mut child, stdout.as_ref(), stderr.as_ref())
        else {
            return Err(GitFailure::Timeout);
        };
        if !status.success() {
            return Err(GitFailure::Exit(clean_reason(&String::from_utf8_lossy(
                &stderr,
            ))));
        }
        if stdout_overflow {
            return Err(GitFailure::OutputTooLarge);
        }
        Ok(String::from_utf8_lossy(&stdout).into_owned())
    }

    fn command(&self, folder: &Path, is_read: bool) -> Command {
        let mut command = Command::new(&self.program);
        command.args(["-c", NO_HOOKS_PATH]);
        for switch in DISABLED_HOOK_EVENTS {
            command.args(["-c", switch]);
        }
        command
            // A configured file-system monitor is a program git would start.
            .args(["-c", "core.fsmonitor=false", "-C"])
            .arg(folder)
            .env_clear()
            .envs(&self.environment)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if is_read {
            command
                .env(NO_OPTIONAL_LOCKS.0, NO_OPTIONAL_LOCKS.1)
                .env(NO_LAZY_FETCH.0, NO_LAZY_FETCH.1);
        }
        own_process_group(&mut command);
        command
    }
}

/// The user's environment without any variable that would change which
/// repository git reads or make it ask for a credential, plus the fixed
/// variables.
#[must_use]
pub fn child_environment(login_environment: &Environment) -> Environment {
    let mut environment: Environment = login_environment
        .iter()
        .filter(|(name, _)| {
            !name.to_ascii_uppercase().starts_with(GIT_VARIABLE_PREFIX)
                && !ASKPASS_VARIABLES.contains(&name.to_ascii_uppercase().as_str())
        })
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    for (name, value) in FIXED_VARIABLES {
        environment.insert(name.to_owned(), value.to_owned());
    }
    environment
}

/// The major and minor version of `git version` output, such as
/// `git version 2.45.1.windows.1` or `git version 2.39.3 (Apple Git-145)`.
#[must_use]
pub fn parse_version(output: &str) -> Option<(u32, u32)> {
    let version = output.trim().strip_prefix("git version ")?;
    let mut parts = version.split(['.', ' ']);
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

/// git's error text as a user may read it: every `user:password@` part of a
/// URL removed, on one line, at most 500 characters.
#[must_use]
pub fn clean_reason(stderr: &str) -> String {
    let one_line = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    let cleaned = without_url_credentials(&one_line);
    // The repository partly controls this text; a control or bidirectional
    // format character could change how the client shows it.
    cleaned
        .chars()
        .filter(|character| !character.is_control() && !is_bidi_format(*character))
        .take(REASON_MAX_CHARS)
        .collect()
}

fn is_bidi_format(character: char) -> bool {
    matches!(character, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

fn without_url_credentials(text: &str) -> String {
    const SCHEME_END: &str = "://";
    let mut cleaned = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(SCHEME_END) {
        let authority_start = start + SCHEME_END.len();
        cleaned.push_str(&rest[..authority_start]);
        rest = &rest[authority_start..];
        let authority_end = rest.find(['/', ' ', '\'', '"']).unwrap_or(rest.len());
        if let Some(at) = rest[..authority_end].rfind('@') {
            rest = &rest[at + 1..];
        }
    }
    cleaned.push_str(rest);
    cleaned
}

type ReaderResult = mpsc::Receiver<(Vec<u8>, bool)>;

/// Reads the whole pipe, so the child never blocks on a full pipe, and
/// keeps at most `limit` bytes; the flag says that more came.
fn bounded_reader(mut pipe: impl Read + Send + 'static, limit: usize) -> ReaderResult {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut kept = Vec::new();
        let mut overflow = false;
        let mut chunk = [0_u8; READ_CHUNK_BYTES];
        while let Ok(count) = pipe.read(&mut chunk) {
            if count == 0 {
                break;
            }
            let room = limit.saturating_sub(kept.len());
            kept.extend_from_slice(&chunk[..count.min(room)]);
            overflow |= count > room;
        }
        // The receiver is gone when the run timed out.
        let _ = sender.send((kept, overflow));
    });
    receiver
}

/// The output of a git that has exited. A helper it started may still hold
/// a pipe, which would never end the read: after a short drain the whole
/// group is ended, so the pipes close. Standard output is `None` when it
/// still did not end; standard error then reads as empty, because only a
/// failure's reason needs it.
fn exited_output(
    child: &mut Child,
    stdout: Option<&ReaderResult>,
    stderr: Option<&ReaderResult>,
) -> (Option<(Vec<u8>, bool)>, Vec<u8>) {
    let mut stdout_read = drained(stdout);
    let mut stderr_read = drained(stderr);
    if stdout_read.is_none() || stderr_read.is_none() {
        end_tree(child);
        stdout_read = stdout_read.or_else(|| drained(stdout));
        stderr_read = stderr_read.or_else(|| drained(stderr));
    }
    (
        stdout_read,
        stderr_read.map_or_else(Vec::new, |(bytes, _)| bytes),
    )
}

/// The reader's bytes, or `None` when the pipe is still open after the
/// drain. A pipe that was never opened reads as empty.
fn drained(reader: Option<&ReaderResult>) -> Option<(Vec<u8>, bool)> {
    let Some(reader) = reader else {
        return Some((Vec::new(), false));
    };
    reader.recv_timeout(PIPE_DRAIN).ok()
}

fn wait_until(child: &mut Child, deadline: Instant) -> Option<std::process::ExitStatus> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(EXIT_POLL),
            _ => return None,
        }
    }
}

#[cfg(unix)]
fn own_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    // A group of its own, so a timeout ends git with every filter or
    // helper it started.
    command.process_group(0);
}

#[cfg(windows)]
fn own_process_group(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

/// Ends git and everything it started. The terminal module has the same
/// walk; it is repeated here to keep the module boundary.
#[cfg(unix)]
fn end_tree(child: &mut Child) {
    use nix::sys::signal::{Signal, killpg};
    use nix::unistd::Pid;
    if let Ok(raw) = i32::try_from(child.id()) {
        // A group that is already gone is the state the caller wants.
        let _ = killpg(Pid::from_raw(raw), Signal::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(windows)]
fn end_tree(child: &mut Child) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    // `taskkill /T` walks the tree itself; a tree already gone makes it
    // fail, which is fine. A helper whose parent already exited is not in
    // that tree and can outlive the timeout; only Unix ends the whole group.
    // The full path, so no folder on PATH supplies another program.
    let program = std::env::var_os("SystemRoot").map_or_else(
        || PathBuf::from("taskkill.exe"),
        |root| PathBuf::from(root).join("System32").join("taskkill.exe"),
    );
    let _ = Command::new(program)
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
#[path = "git_runner.tests.rs"]
mod tests;
