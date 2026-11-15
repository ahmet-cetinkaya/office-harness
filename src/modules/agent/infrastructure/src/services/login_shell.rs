// SPDX-License-Identifier: AGPL-3.0-or-later

//! Asking a login shell for its environment, the way VS Code does: run the
//! shell with a command that prints the environment between two markers, so
//! noise from the shell's start files does not matter.

use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use nix::sys::signal::{Signal, killpg};
use nix::unistd::{Pid, Uid, User};

use crate::services::login_environment::Environment;

const MARKER: &str = "__OFFICE_HARNESS_ENVIRONMENT__";
const READ_CHUNK_BYTES: usize = 8 * 1024;
const EXIT_POLL: Duration = Duration::from_millis(10);
const NAME_SEPARATOR: char = '=';

/// The account record of the user that runs the server.
pub struct Account {
    pub name: String,
    pub home: PathBuf,
    pub shell: PathBuf,
}

impl Account {
    /// The variables that a shell needs to find the user's start files when
    /// the server's own environment has none.
    pub fn variables(&self) -> Vec<(String, String)> {
        vec![
            ("HOME".to_owned(), self.home.to_string_lossy().into_owned()),
            ("USER".to_owned(), self.name.clone()),
            ("LOGNAME".to_owned(), self.name.clone()),
        ]
    }
}

pub fn current_account() -> Option<Account> {
    let user = User::from_uid(Uid::current()).ok().flatten()?;
    Some(Account {
        name: user.name,
        home: user.dir,
        shell: user.shell,
    })
}

/// Runs `shell` with the arguments and `-c`, with exactly `base` as its
/// environment. Returns `None` when the shell fails to start, ends without
/// the markers, or does not answer within `timeout`; a shell that does not
/// answer is ended together with everything it started.
pub fn run_login_shell(
    shell: &Path,
    arguments: &[&str],
    base: &Environment,
    timeout: Duration,
) -> Option<Environment> {
    let script = format!("printf '%s' '{MARKER}'; env -0; printf '%s' '{MARKER}'");
    let deadline = Instant::now() + timeout;
    let mut child = Command::new(shell)
        .args(arguments)
        .arg("-c")
        .arg(script)
        .env_clear()
        .envs(base)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        // A group of its own, so the whole group can end on a timeout. A
        // session of its own would need `pre_exec`, which is `unsafe`, and
        // `Command::setsid` is not stable yet; the shell reads no terminal
        // because its input is null.
        .process_group(0)
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut output = Vec::new();
        let mut chunk = [0_u8; READ_CHUNK_BYTES];
        // Stops at the closing marker, so a daemon that a start file left
        // running with the pipe open does not look like a hang.
        while let Ok(count) = stdout.read(&mut chunk) {
            output.extend_from_slice(&chunk[..count]);
            if count == 0 || parse_marked_environment(&output).is_some() {
                break;
            }
        }
        // The receiver is gone when the caller timed out.
        let _ = sender.send(output);
    });
    let answer = receiver
        .recv_timeout(timeout)
        .ok()
        .and_then(|output| parse_marked_environment(&output));
    // The answer is in, but a start file may keep the shell alive after it.
    // The whole run, the wait for the exit included, stays within `timeout`.
    wait_for_exit_until(&mut child, deadline);
    answer
}

/// Waits for the shell to end. When it is still there at the deadline, the
/// whole group ends.
fn wait_for_exit_until(child: &mut Child, deadline: Instant) {
    while let Ok(None) = child.try_wait() {
        if Instant::now() >= deadline {
            end_group(child);
            return;
        }
        thread::sleep(EXIT_POLL);
    }
}

fn end_group(child: &mut Child) {
    if let Ok(raw) = i32::try_from(child.id()) {
        // A group that is already gone is the state that the caller wants.
        let _ = killpg(Pid::from_raw(raw), Signal::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// The variables between the first two markers of the shell's output, or
/// `None` when the output does not hold both markers and a variable.
pub fn parse_marked_environment(output: &[u8]) -> Option<Environment> {
    let text = String::from_utf8_lossy(output);
    let start = text.find(MARKER)? + MARKER.len();
    let end = start + text[start..].find(MARKER)?;
    let variables: Environment = text[start..end]
        .split('\0')
        .filter_map(|entry| entry.split_once(NAME_SEPARATOR))
        .filter(|(name, _)| !name.is_empty())
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect();
    (!variables.is_empty()).then_some(variables)
}

/// The marker, for the tests' stand-in shells.
#[cfg(test)]
pub const MARKER_FOR_TESTS: &str = MARKER;

#[cfg(test)]
#[path = "login_shell.tests.rs"]
mod tests;
