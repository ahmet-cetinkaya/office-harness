// SPDX-License-Identifier: AGPL-3.0-or-later

//! The handle for a system without a process handle: macOS, and Linux before
//! 5.3. It holds the process id and the start time seen at open. Before each
//! end it reads the start time again and ends nothing when it differs. A
//! process that exits and whose id is reused in the short time between the
//! read and the signal can still be ended wrongly; the spec accepts that risk
//! for these systems only (A10).

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

pub struct PidHandle {
    pid: u32,
    started_at: u64,
}

impl PidHandle {
    /// `None` when the process cannot be read now: it is gone, or the OS
    /// refuses the read.
    pub fn open(pid: u32) -> Option<Box<dyn ProcessHandle>> {
        let identity = read_identity(pid).ok()?;
        Some(Box::new(Self {
            pid,
            started_at: identity.started_at,
        }))
    }

    /// Reads again, and tells whether the id still names the process that this
    /// handle was opened on.
    fn is_same_process(&self) -> bool {
        read_identity(self.pid).is_ok_and(|live| live.started_at == self.started_at)
    }

    fn end_with(&self, signal: Signal) -> Result<EndOutcome, String> {
        if !self.is_same_process() {
            return Ok(EndOutcome::Gone);
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
            if !self.is_same_process() {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(EXIT_POLL_INTERVAL);
        }
    }
}
