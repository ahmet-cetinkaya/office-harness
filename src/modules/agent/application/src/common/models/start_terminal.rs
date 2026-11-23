// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use agent_domain::value_objects::SessionId;
use kernel::AgentId;

use super::SignIn;

/// Whether a session of the program gets the office settings file with the
/// status hooks (0024).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusHooksRequest {
    /// The CLI has no status hooks.
    Unsupported,
    /// The command or the office additions already pass a settings option
    /// of their own (0024/FR-004). The text says so for the agent's details.
    Refused(String),
    /// The CLI has status hooks and nothing stands in the way. The host
    /// still checks the setting and the CLI version.
    Wanted,
}

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
    pub status_hooks: StatusHooksRequest,
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
    /// The arguments that name the office settings file of 0024, empty when
    /// the session gets none. The launcher adds them to the command line.
    pub hook_arguments: Vec<String>,
    /// What the agent's details say of the status source until the first
    /// report arrives, or why the session runs without hooks (0024/FR-019).
    pub hook_note: Option<String>,
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
    /// True when `arguments` carry the office settings file of 0024, which
    /// the host makes right before the spawn.
    pub has_hook_arguments: bool,
    /// The note of `PreparedLaunch::hook_note`.
    pub hook_note: Option<String>,
    /// The CLI's sign-in knowledge: the first seconds are watched for its
    /// sign-in screen words, if it has any (0019/FR-005).
    pub sign_in: SignIn,
}
