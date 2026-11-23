// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use crate::common::models::ProcessIdentity;

/// Why a process identity could not be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadError {
    /// The process has exited or is a zombie.
    NotRunning,
    /// The OS refused a value, or the command line is empty. The process is
    /// never a match then.
    NotReadable,
}

/// What an end call did to the process behind a handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndOutcome {
    /// The request or the force reached the process.
    Delivered,
    /// The process behind the handle had already gone; nothing was signaled.
    Gone,
    /// This OS has no polite end request for the process, so the caller goes
    /// straight to `force_end` (Windows, for a process without a console).
    Unsupported,
}

/// Reads processes and opens handles to them. Every end goes through a
/// `ProcessHandle`; the port has no call that ends a process by its id.
pub trait ProcessInspector: Send + Sync {
    /// Opens a handle to the process with this id.
    ///
    /// # Errors
    /// `ReadError::NotRunning` only when the OS says that no such process
    /// exists. Every other failure (access denied, an unreadable process, an
    /// id that cannot name a leftover) is `ReadError::NotReadable`, so a
    /// process that may still run is never taken for one that is gone.
    fn open(&self, pid: u32) -> Result<Box<dyn ProcessHandle>, ReadError>;

    /// The machine run now, in the form of `ProcessIdentity::boot`.
    fn boot(&self) -> Option<String>;

    /// Every process that descends from `root` through parent links, never
    /// `root` itself, up to a fixed cap.
    fn descendants(&self, root: u32) -> Vec<u32>;
}

/// One process, held. Where the OS has a handle for a process (Linux 5.3 and
/// later, Windows), the identity is read on it and the end goes to it, so an
/// id that the OS hands out again is never signaled. Where it has none (macOS,
/// older Linux), the handle holds the id and reads again just before it ends.
pub trait ProcessHandle: Send {
    /// Reads the identity of the process behind the handle.
    ///
    /// # Errors
    /// `ReadError::NotRunning` when the process has exited, and
    /// `ReadError::NotReadable` when a value cannot be read.
    fn identity(&self) -> Result<ProcessIdentity, ReadError>;

    /// Asks the process, and only that process, to end.
    ///
    /// # Errors
    /// The text of the OS error when the OS refuses.
    fn request_end(&self) -> Result<EndOutcome, String>;

    /// Ends the process, and only that process, without asking.
    ///
    /// # Errors
    /// The text of the OS error when the OS refuses.
    fn force_end(&self) -> Result<EndOutcome, String>;

    /// Waits up to `timeout` for the process to exit. `true` when it has.
    fn wait_for_exit(&self, timeout: Duration) -> bool;
}

#[cfg(test)]
#[path = "process_inspector.tests.rs"]
mod tests;
