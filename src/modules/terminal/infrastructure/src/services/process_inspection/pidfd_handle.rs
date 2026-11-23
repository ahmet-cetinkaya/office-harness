// SPDX-License-Identifier: AGPL-3.0-or-later

//! A process handle on Linux 5.3 and later. The pidfd names one process for
//! its whole life, so a process id that the kernel hands out again is never
//! signaled.

use std::os::fd::{AsFd, OwnedFd};
use std::time::{Duration, Instant};

use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use terminal_application::common::interfaces::{EndOutcome, ProcessHandle, ReadError};
use terminal_application::common::models::ProcessIdentity;

use super::{pidfd, proc_reader};

pub struct PidfdHandle {
    pidfd: OwnedFd,
    pid: u32,
}

impl PidfdHandle {
    pub fn new(pidfd: OwnedFd, pid: u32) -> Self {
        Self { pidfd, pid }
    }

    /// A pidfd becomes readable when its process exits. `PollTimeout::ZERO`
    /// asks without waiting.
    fn exited_within(&self, timeout: PollTimeout) -> bool {
        let mut descriptors = [PollFd::new(self.pidfd.as_fd(), PollFlags::POLLIN)];
        matches!(poll(&mut descriptors, timeout), Ok(ready) if ready > 0)
    }

    fn end_with(&self, signal: libc::c_int) -> Result<EndOutcome, String> {
        if self.exited_within(PollTimeout::ZERO) {
            return Ok(EndOutcome::Gone);
        }
        match pidfd::send_signal(&self.pidfd, signal) {
            Ok(()) => Ok(EndOutcome::Delivered),
            Err(error) if error.raw_os_error() == Some(libc::ESRCH) => Ok(EndOutcome::Gone),
            Err(error) => Err(error.to_string()),
        }
    }
}

impl ProcessHandle for PidfdHandle {
    /// Reads `/proc`, then asks the pidfd whether the process has exited. The
    /// read may have seen a new process that took the id after the exit, so
    /// an exit seen after the read makes the answer "not running".
    fn identity(&self) -> Result<ProcessIdentity, ReadError> {
        let identity = proc_reader::read_identity(self.pid);
        if self.exited_within(PollTimeout::ZERO) {
            return Err(ReadError::NotRunning);
        }
        identity
    }

    fn request_end(&self) -> Result<EndOutcome, String> {
        self.end_with(libc::SIGTERM)
    }

    fn force_end(&self) -> Result<EndOutcome, String> {
        self.end_with(libc::SIGKILL)
    }

    fn wait_for_exit(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let wait = PollTimeout::try_from(remaining).unwrap_or(PollTimeout::MAX);
            if self.exited_within(wait) {
                return true;
            }
            // A poll that an interrupt cut short comes back early; the
            // deadline decides, not the poll.
            if Instant::now() >= deadline {
                return false;
            }
        }
    }
}
