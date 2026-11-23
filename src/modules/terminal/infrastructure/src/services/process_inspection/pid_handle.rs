// SPDX-License-Identifier: AGPL-3.0-or-later

//! The handle for a system without a process handle: macOS, and Linux before
//! 5.3. It holds the process id and the start time seen at open. Before each
//! end it reads the start time again and ends nothing when it differs or
//! cannot be read. A process that exits and whose id is reused in the short
//! time between the read and the signal can still be ended wrongly; the spec
//! accepts that risk for these systems only (A10).

use std::thread;
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use terminal_application::common::interfaces::{EndOutcome, ProcessHandle, ReadError};
use terminal_application::common::models::ProcessIdentity;

#[cfg(target_os = "linux")]
use super::proc_reader::read_identity;
#[cfg(target_os = "macos")]
use super::sysinfo_reader::read_identity;

/// How often `wait_for_exit` reads the process again.
const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(20);
/// Why no signal goes to a process that the read just before it could not
/// read: the office cannot tell that it is still the stored process.
const NOT_READABLE_BEFORE_END: &str = "the process cannot be read before the end request";

pub struct PidHandle {
    pid: u32,
    started_at: u64,
}

impl PidHandle {
    /// The error of the first read: `NotRunning` when the process is gone,
    /// `NotReadable` when the OS refuses the read.
    pub fn open(pid: u32) -> Result<Box<dyn ProcessHandle>, ReadError> {
        let identity = read_identity(pid)?;
        Ok(Box::new(Self {
            pid,
            started_at: identity.started_at,
        }))
    }

    fn end_with(&self, signal: Signal) -> Result<EndOutcome, String> {
        if let Some(outcome) = outcome_without_signal(&read_identity(self.pid), self.started_at)? {
            return Ok(outcome);
        }
        let Ok(raw) = i32::try_from(self.pid) else {
            return Err(format!("process id {} is out of range", self.pid));
        };
        // To the process id only, never to its group.
        match kill(Pid::from_raw(raw), signal) {
            Ok(()) => Ok(EndOutcome::Delivered),
            Err(Errno::ESRCH) => Ok(EndOutcome::Gone),
            Err(error) => Err(error.to_string()),
        }
    }
}

/// What the read just before a signal decides: `Gone` when the process is
/// gone or the id names another process, an error when it cannot be read (the
/// sweep then records that it could not end it), and nothing when the signal
/// may go.
pub(super) fn outcome_without_signal(
    read: &Result<ProcessIdentity, ReadError>,
    started_at: u64,
) -> Result<Option<EndOutcome>, String> {
    match read {
        Err(ReadError::NotReadable) => Err(NOT_READABLE_BEFORE_END.to_owned()),
        Err(ReadError::NotRunning) => Ok(Some(EndOutcome::Gone)),
        Ok(live) if live.started_at != started_at => Ok(Some(EndOutcome::Gone)),
        Ok(_) => Ok(None),
    }
}

/// True only when the read confirms the process is gone (not running, or a
/// zombie) or the id now names another process. A read that fails for any
/// other reason says nothing about the exit: the process may still run.
pub(super) fn exit_confirmed(read: &Result<ProcessIdentity, ReadError>, started_at: u64) -> bool {
    match read {
        Ok(live) => live.started_at != started_at,
        Err(ReadError::NotRunning) => true,
        Err(ReadError::NotReadable) => false,
    }
}

impl ProcessHandle for PidHandle {
    fn identity(&self) -> Result<ProcessIdentity, ReadError> {
        read_identity(self.pid)
    }

    fn request_end(&self) -> Result<EndOutcome, String> {
        self.end_with(Signal::SIGTERM)
    }

    fn force_end(&self) -> Result<EndOutcome, String> {
        self.end_with(Signal::SIGKILL)
    }

    fn wait_for_exit(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if exit_confirmed(&read_identity(self.pid), self.started_at) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(EXIT_POLL_INTERVAL);
        }
    }
}
