// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use agent_domain::value_objects::SessionId;
use kernel::AgentId;

/// What the terminal host needs to find the program and its environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepareLaunch {
    /// The program as typed in the agent's command.
    pub program: String,
    pub working_folder: PathBuf,
    /// The office additions' variables, applied over the login environment
    /// (FR-020).
    pub environment_additions: Vec<(String, String)>,
    /// The agent and the session this launch is for, so the host can give the
    /// child the call credential of exactly this session (0002).
    pub agent_id: AgentId,
    pub session_id: SessionId,
}

/// The program found on the login `PATH`, and the complete environment the
/// child gets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedLaunch {
    pub program: String,
    pub environment: Vec<(String, String)>,
    /// True for a Windows `.cmd` or `.bat` file, whose arguments cannot carry
    /// shell characters safely (D5).
    pub is_batch_script: bool,
}

/// What the terminal host needs to start one session. The host adds the size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartTerminal {
    pub agent_id: AgentId,
    /// Names the session's output file.
    pub session_id: SessionId,
    pub program: String,
    pub arguments: Vec<String>,
    pub working_folder: PathBuf,
    pub environment: Vec<(String, String)>,
    /// Typed into the terminal after its first output, as a paste.
    pub typed_input: Option<String>,
}
