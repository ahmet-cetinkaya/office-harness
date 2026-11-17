// SPDX-License-Identifier: AGPL-3.0-or-later

//! Ending a child together with everything it started. A child in a PTY leads
//! its own session, so on Unix its process id is also its group id.

#[cfg(unix)]
mod unix {
    use std::collections::{HashMap, VecDeque};
    use std::process::Command;

    use nix::sys::signal::{Signal, kill, killpg};
    use nix::unistd::Pid;

    /// Most processes one walk reports, so a runaway fork tree or a huge
    /// process table cannot make a stop slow or the list unbounded.
    pub(super) const MAX_DESCENDANTS: usize = 4096;

    fn process_id(raw: u32) -> Option<Pid> {
        i32::try_from(raw).ok().map(Pid::from_raw)
    }

    /// Asks the child's whole group to end.
    pub fn request_end(root: u32) {
        if let Some(group) = process_id(root) {
            // A group that is already gone answers "no such process", which
            // is the state that the caller wants.
            let _ = killpg(group, Signal::SIGTERM);
        }
    }

    /// Every process that descends from `root` through parent links. Taken
    /// before the end request, because the survivors of a dead parent move to
    /// another parent and drop out of the chain.
    pub fn descendants(root: u32) -> Vec<u32> {
        let output = Command::new("ps")
            .args(["-A", "-o", "pid=", "-o", "ppid="])
            .output();
        let Ok(output) = output else {
            return Vec::new();
        };
        descendants_in(root, &String::from_utf8_lossy(&output.stdout))
    }

    /// Ends the group and then each descendant that left it, without asking.
    pub fn force_end(root: u32, descendants: &[u32]) {
        if let Some(group) = process_id(root) {
            let _ = killpg(group, Signal::SIGKILL);
        }
        for descendant in descendants.iter().filter_map(|raw| process_id(*raw)) {
            let _ = kill(descendant, Signal::SIGKILL);
        }
    }

    /// Parses `ps` output of `pid ppid` pairs and walks the tree below `root`, up
    /// to `MAX_DESCENDANTS` processes.
    pub(super) fn descendants_in(root: u32, table: &str) -> Vec<u32> {
        let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
        for line in table.lines() {
            let mut columns = line.split_whitespace().map(str::parse::<u32>);
            if let (Some(Ok(pid)), Some(Ok(parent))) = (columns.next(), columns.next()) {
                children.entry(parent).or_default().push(pid);
            }
        }
        let mut found = Vec::new();
        let mut queue = VecDeque::from([root]);
        while let Some(parent) = queue.pop_front() {
            for &child in children.get(&parent).into_iter().flatten() {
                if found.len() == MAX_DESCENDANTS {
                    return found;
                }
                found.push(child);
                queue.push_back(child);
            }
        }
        found
    }
}

#[cfg(windows)]
mod windows {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    /// Windows has no end request for a console child that works from
    /// outside; the caller writes the interrupt key to the PTY instead.
    pub fn request_end(_root: u32) {}

    /// `taskkill /T` walks the tree itself, so no list is needed.
    pub fn descendants(_root: u32) -> Vec<u32> {
        Vec::new()
    }

    pub fn force_end(root: u32, _descendants: &[u32]) {
        // A tree that is already gone makes taskkill fail, which is fine.
        let _ = Command::new("taskkill")
            .args(["/PID", &root.to_string(), "/T", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }
}

#[cfg(unix)]
pub use unix::{descendants, force_end, request_end};
#[cfg(windows)]
pub use windows::{descendants, force_end, request_end};

#[cfg(all(test, unix))]
#[path = "process_tree.tests.rs"]
mod tests;
