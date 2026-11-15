// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use kernel::AgentId;

/// What the terminal host needs to start one agent. The host adds the
/// environment, the size, and the output key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartTerminal {
    pub agent_id: AgentId,
    pub program: String,
    pub arguments: Vec<String>,
    pub working_folder: PathBuf,
}
