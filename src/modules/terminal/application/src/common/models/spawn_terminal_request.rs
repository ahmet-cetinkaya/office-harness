// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use kernel::AgentId;
use terminal_domain::value_objects::{OutputKey, TerminalSize};

/// What the runtime needs to start one child. The environment is complete:
/// the child gets exactly these variables.
#[derive(Debug, Clone)]
pub struct SpawnTerminalRequest {
    pub agent_id: AgentId,
    pub program: String,
    pub arguments: Vec<String>,
    pub working_folder: PathBuf,
    pub environment: Vec<(String, String)>,
    pub size: TerminalSize,
    pub output_key: OutputKey,
}
