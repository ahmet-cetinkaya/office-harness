// SPDX-License-Identifier: AGPL-3.0-or-later

//! The process inspection adapter of the 0017 orphan sweep. It reads the
//! identity of a process and ends it through a handle: a pidfd on Linux 5.3
//! and later, a process handle on Windows. macOS and older Linux have no
//! handle, so there the handle holds the process id and reads again just
//! before it ends (spec A10).

use std::collections::{HashMap, HashSet, VecDeque};
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, Ordering};

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use terminal_application::common::interfaces::{ProcessHandle, ProcessInspector, ReadError};

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod pid_handle;
#[cfg(target_os = "linux")]
mod pidfd;
#[cfg(target_os = "linux")]
mod pidfd_handle;
#[cfg(target_os = "linux")]
mod proc_reader;
#[cfg(any(target_os = "macos", all(test, target_os = "linux")))]
mod sysinfo_reader;
#[cfg(windows)]
mod windows_handle;

/// Most processes one walk reports, so a runaway fork tree or a huge process
/// table cannot make a read slow or the list unbounded.
const MAX_DESCENDANTS: usize = 4096;

/// Process ids up to this one are never opened.
const MAX_UNOPENABLE_PID: u32 = 1;

#[cfg(any(target_os = "linux", target_os = "macos"))]
const PID_FALLBACK_NOTICE: &str = "office-harness: no process handle on this system; a leftover process is read again and then ended by its id";

/// Reads processes of this machine and opens handles to them.
pub struct SystemProcessInspector {
    /// False on a kernel without `pidfd_open`, or when a test forces the
    /// fallback path.
    #[cfg(target_os = "linux")]
    pidfd_enabled: AtomicBool,
}

impl SystemProcessInspector {
    #[must_use]
    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        eprintln!("{PID_FALLBACK_NOTICE}");
        Self {
            #[cfg(target_os = "linux")]
            pidfd_enabled: AtomicBool::new(true),
        }
    }

    /// An inspector that treats `pidfd_open` as missing, as on a kernel before
    /// 5.3, so the read-then-end-by-id path can be tested on a newer one.
    #[cfg(target_os = "linux")]
    #[must_use]
    pub fn without_pidfd() -> Self {
        eprintln!("{PID_FALLBACK_NOTICE}");
        Self {
            pidfd_enabled: AtomicBool::new(false),
        }
    }

    #[cfg(target_os = "linux")]
    fn open_handle(&self, pid: u32) -> Result<Box<dyn ProcessHandle>, ReadError> {
        if self.pidfd_enabled.load(Ordering::Relaxed) {
            match pidfd::open(pid) {
                Ok(descriptor) => {
                    return Ok(Box::new(pidfd_handle::PidfdHandle::new(descriptor, pid)));
                }
                Err(error) if error.raw_os_error() == Some(libc::ENOSYS) => {
                    if self.pidfd_enabled.swap(false, Ordering::Relaxed) {
                        eprintln!("{PID_FALLBACK_NOTICE}");
                    }
                }
                Err(error) if error.raw_os_error() == Some(libc::ESRCH) => {
                    return Err(ReadError::NotRunning);
                }
                Err(_) => return Err(ReadError::NotReadable),
            }
        }
        pid_handle::PidHandle::open(pid)
    }

    #[cfg(target_os = "macos")]
    #[allow(clippy::unused_self, reason = "one shape of open_handle on every OS")]
    fn open_handle(&self, pid: u32) -> Result<Box<dyn ProcessHandle>, ReadError> {
        pid_handle::PidHandle::open(pid)
    }

    #[cfg(windows)]
    #[allow(clippy::unused_self, reason = "one shape of open_handle on every OS")]
    fn open_handle(&self, pid: u32) -> Result<Box<dyn ProcessHandle>, ReadError> {
        windows_handle::WindowsHandle::open(pid)
    }
}

impl Default for SystemProcessInspector {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessInspector for SystemProcessInspector {
    fn open(&self, pid: u32) -> Result<Box<dyn ProcessHandle>, ReadError> {
        // Id 0 would make `kill` signal the caller's process group, and id 1
        // is the init process; neither can be a leftover of the office.
        if pid <= MAX_UNOPENABLE_PID {
            return Err(ReadError::NotReadable);
        }
        self.open_handle(pid)
    }

    fn boot(&self) -> Option<String> {
        machine_run()
    }

    fn descendants(&self, root: u32) -> Vec<u32> {
        let Ok(mut system) = System::new() else {
            return Vec::new();
        };
        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing(),
        );
        let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut started_at: HashMap<u32, u64> = HashMap::new();
        for (pid, process) in system.processes() {
            started_at.insert(pid.as_u32(), process.start_time());
            if let Some(parent) = process.parent() {
                children
                    .entry(parent.as_u32())
                    .or_default()
                    .push(pid.as_u32());
            }
        }
        walk_children(root, &children, &started_at)
    }
}

/// The machine run in the form of `ProcessIdentity::boot`.
#[cfg(target_os = "linux")]
fn machine_run() -> Option<String> {
    proc_reader::boot_id()
}

/// The boot time in whole seconds as text.
#[cfg(not(target_os = "linux"))]
fn machine_run() -> Option<String> {
    System::boot_time().ok().map(|seconds| seconds.to_string())
}

/// Walks the tree below `root` breadth first, up to `MAX_DESCENDANTS`. A pid
/// is taken once, so a loop in the table cannot hold the walk.
///
/// A child is taken only when it started at or after its parent. Windows does
/// not update the parent field when the parent exits, and it hands out the id
/// again quickly, so a stale link can name an unrelated process that is older
/// than the process that now holds the id. Such a child and everything below
/// it is skipped. `started_at` is `sysinfo`'s start time (whole seconds), the
/// same unit for both sides of the comparison; equal values are accepted.
fn walk_children(
    root: u32,
    children: &HashMap<u32, Vec<u32>>,
    started_at: &HashMap<u32, u64>,
) -> Vec<u32> {
    let start_of = |pid: u32| started_at.get(&pid).copied().unwrap_or_default();
    let mut found = Vec::new();
    let mut seen = HashSet::from([root]);
    let mut queue = VecDeque::from([root]);
    while let Some(parent) = queue.pop_front() {
        for &child in children.get(&parent).into_iter().flatten() {
            if found.len() == MAX_DESCENDANTS {
                return found;
            }
            if start_of(child) >= start_of(parent) && seen.insert(child) {
                found.push(child);
                queue.push_back(child);
            }
        }
    }
    found
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
