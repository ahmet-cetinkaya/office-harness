// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;

use crate::common::models::{ProbeError, SessionProcess};

/// Reads the processes of a session. The host app implements it with the
/// terminal module (the bridge). Operations block.
pub trait ProcessProbe: Send + Sync {
    /// The id of the CLI process of the agent's running terminal.
    fn cli_process_id(&self, agent_id: AgentId) -> Option<u32>;

    /// Reads the identity of the process with this id.
    ///
    /// # Errors
    /// `NotRunning` when no such process runs, `NotReadable` when a value
    /// cannot be read.
    fn read(&self, pid: u32) -> Result<SessionProcess, ProbeError>;

    /// The ids of every process that descends from `root`, never `root`.
    fn descendants(&self, root: u32) -> Vec<u32>;
}
