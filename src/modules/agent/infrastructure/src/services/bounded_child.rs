// SPDX-License-Identifier: AGPL-3.0-or-later

//! A child process with bounded output and a deadline that ends its whole
//! tree: shared by the git runner (0010 D1) and the sign-in status command
//! (0019 D6).

use std::io::Read;
#[cfg(windows)]
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const READ_CHUNK_BYTES: usize = 8 * 1024;
const EXIT_POLL: Duration = Duration::from_millis(10);
/// How long a pipe may stay open after the child exited: the reader needs
/// only to take what the child already wrote.
const PIPE_DRAIN: Duration = Duration::from_millis(500);

pub(crate) type ReaderResult = mpsc::Receiver<(Vec<u8>, bool)>;

/// Reads the whole pipe, so the child never blocks on a full pipe, and
/// keeps at most `limit` bytes; the flag says that more came.
pub(crate) fn bounded_reader(mut pipe: impl Read + Send + 'static, limit: usize) -> ReaderResult {
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

/// The output of a child that has exited. A helper it started may still hold
/// a pipe, which would never end the read: after a short drain the whole
/// group is ended, so the pipes close. Standard output is `None` when it
/// still did not end; standard error then reads as empty, because only a
/// failure's reason needs it.
pub(crate) fn exited_output(
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

/// Waits until the child has exited or the deadline passes. The child stays
/// unreaped where the system can look without reaping, so its pid cannot be
/// reused before `end_tree` signals the group; the caller then takes the
/// status with `Child::wait`.
pub(crate) fn wait_until(child: &mut Child, deadline: Instant) -> bool {
    loop {
        if has_exited(child) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(EXIT_POLL);
    }
}

#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
fn has_exited(child: &Child) -> bool {
    use nix::sys::wait::{Id, WaitPidFlag, WaitStatus, waitid};
    use nix::unistd::Pid;
    let Ok(raw) = i32::try_from(child.id()) else {
        return true;
    };
    let flags = WaitPidFlag::WEXITED | WaitPidFlag::WNOHANG | WaitPidFlag::WNOWAIT;
    // An error means there is nothing left to wait for.
    waitid(Id::Pid(Pid::from_raw(raw)), flags) != Ok(WaitStatus::StillAlive)
}

/// No system call here looks at an exit without reaping it, so a reused pid
/// stays possible on these systems.
#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "freebsd")))]
fn has_exited(child: &mut Child) -> bool {
    !matches!(child.try_wait(), Ok(None))
}

#[cfg(unix)]
pub(crate) fn own_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    // A group of its own, so a timeout ends git with every filter or
    // helper it started.
    command.process_group(0);
}

#[cfg(windows)]
pub(crate) fn own_process_group(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

/// Ends the child and everything it started. The group is signalled before
/// the child is reaped, so a pid that was reused cannot be hit. The terminal
/// module has the same walk; it is repeated here to keep the module boundary.
#[cfg(unix)]
pub(crate) fn end_tree(child: &mut Child) {
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
pub(crate) fn end_tree(child: &mut Child) {
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
#[path = "bounded_child.tests.rs"]
mod tests;
