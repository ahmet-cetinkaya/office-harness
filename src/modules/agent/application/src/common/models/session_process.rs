// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::value_objects::SessionId;

/// What names one run of one process of a session (0017): the process id, the
/// start time, the command line, and the machine run. The units of the start
/// time and of the machine run belong to the OS adapter; this module only
/// stores and compares them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionProcess {
    pub pid: u32,
    pub started_at: u64,
    /// SHA-256 of the whole command line.
    pub command_hash: [u8; 32],
    /// The command line cut to a length the store accepts, as the adapter read it.
    pub command_display: String,
    pub boot: String,
}

/// Whether a stored process is the session's CLI or one of its helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessKind {
    Cli,
    Helper,
}

/// A session process as the store keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredProcess {
    pub id: i64,
    pub session_id: SessionId,
    pub kind: ProcessKind,
    pub process: SessionProcess,
}

/// What the sweep of an unclean stop found for one stored process (0017/D8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepOutcome {
    /// The sweep ended it, with or without force.
    Ended,
    CouldNotEnd,
    /// Left alone because it could not be read.
    NotReadable,
    /// Left alone because the boot times differ; it may be the same process.
    BootTimeChanged,
    /// Left alone because it is not running or is not the stored process.
    Left,
}

/// The sweep results of one session, as `resume.plan` needs them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SweepSummary {
    pub ended: u32,
    pub could_not_end: u32,
    pub not_readable: u32,
    pub boot_time_changed: u32,
    /// Rows the sweep did not close, for example after a disk failure.
    pub still_open: u32,
    /// Every stored process of the session.
    pub stored: u32,
    /// The helper limit stopped the office from storing every helper.
    pub helpers_capped: bool,
}

/// Why a process could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeError {
    /// The process has exited or is a zombie.
    NotRunning,
    /// A value could not be read, or the command line is empty for now.
    NotReadable,
}
