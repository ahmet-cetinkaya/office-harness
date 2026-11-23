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

/// Why a process could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeError {
    /// The process has exited or is a zombie.
    NotRunning,
    /// A value could not be read, or the command line is empty for now.
    NotReadable,
}
