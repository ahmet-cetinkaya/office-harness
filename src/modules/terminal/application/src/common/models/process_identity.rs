// SPDX-License-Identifier: AGPL-3.0-or-later

use sha2::{Digest, Sha256};

/// Characters of the command line that the store keeps for display. The match
/// uses the hash of the whole line, so the cap never weakens it.
pub const COMMAND_DISPLAY_LIMIT: usize = 1_000;

/// Two boot times that differ by no more than this many seconds name the same
/// machine run; a clock correction can move the boot time by a little.
pub const BOOT_TIME_TOLERANCE_SECONDS: u64 = 2;

/// What names one run of one process: the process id, the start time, the
/// command line, and the machine run. A process id alone is not enough,
/// because the OS hands ids out again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessIdentity {
    pub pid: u32,
    /// The start time in the unit of the OS: clock ticks after boot on Linux,
    /// whole seconds on macOS, 100 ns FILETIME units on Windows. Only the
    /// adapter writes and reads it, so the unit never crosses a boundary.
    pub started_at: u64,
    /// SHA-256 of the whole command line.
    pub command_hash: [u8; 32],
    /// The first `COMMAND_DISPLAY_LIMIT` characters of the command line, raw.
    pub command_display: String,
    /// The machine run: `boot_id` on Linux, the boot time in seconds as text
    /// on macOS and Windows.
    pub boot: String,
}

/// The first value that differs between a stored identity and a live one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mismatch {
    /// Another `boot_id`: the machine started again since the identity was stored.
    MachineRestarted,
    OtherStartTime,
    OtherCommandLine,
    /// Everything else matches but the boot times differ by more than the
    /// tolerance. This is not a proof of another process, so it is not a match
    /// and not an end either.
    BootTimeChanged,
}

#[derive(PartialEq, Eq)]
enum BootDifference {
    None,
    Restarted,
    TimeChanged,
}

impl ProcessIdentity {
    /// Builds the identity from the command line as the reader returns it.
    #[must_use]
    pub fn new(pid: u32, started_at: u64, command_line: &str, boot: String) -> Self {
        Self {
            pid,
            started_at,
            command_hash: Sha256::digest(command_line.as_bytes()).into(),
            command_display: command_line.chars().take(COMMAND_DISPLAY_LIMIT).collect(),
            boot,
        }
    }

    /// `None` when `live` is the same process run, else the first value that
    /// differs, in the order of the plan (D2).
    #[must_use]
    pub fn mismatch(&self, live: &Self) -> Option<Mismatch> {
        let boot = compare_boot(&self.boot, &live.boot);
        if boot == BootDifference::Restarted {
            return Some(Mismatch::MachineRestarted);
        }
        if self.started_at != live.started_at {
            return Some(Mismatch::OtherStartTime);
        }
        if self.command_hash != live.command_hash {
            return Some(Mismatch::OtherCommandLine);
        }
        (boot == BootDifference::TimeChanged).then_some(Mismatch::BootTimeChanged)
    }
}

/// Boot times are whole seconds as text and get the tolerance; any other
/// text is a `boot_id` and is compared for equality.
fn compare_boot(stored: &str, live: &str) -> BootDifference {
    match (stored.parse::<i64>(), live.parse::<i64>()) {
        (Ok(stored_seconds), Ok(live_seconds)) => {
            if stored_seconds.abs_diff(live_seconds) > BOOT_TIME_TOLERANCE_SECONDS {
                BootDifference::TimeChanged
            } else {
                BootDifference::None
            }
        }
        _ if stored == live => BootDifference::None,
        _ => BootDifference::Restarted,
    }
}

#[cfg(test)]
#[path = "process_identity.tests.rs"]
mod tests;
