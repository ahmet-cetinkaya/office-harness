// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;

/// What an adapter needs to build one command line.
#[derive(Debug, Clone, Copy)]
pub struct SessionPlan<'a> {
    /// The arguments typed in the agent's command, after the program.
    pub fixed_arguments: &'a [String],
    /// The office additions' flags.
    pub flags: &'a [String],
    /// Absent on a continue: the prompt goes only to a new session.
    pub startup_prompt: Option<&'a str>,
    /// False when the program is a Windows batch script: the prompt is then
    /// typed in after the start, never put on the command line (D5).
    pub prompt_by_argument: bool,
    /// An id for a CLI that accepts one at the start.
    pub proposed_cli_session_id: &'a str,
}

/// The command line to start, and what goes with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCommand {
    pub arguments: Vec<String>,
    /// The CLI's session id when the start fixes it.
    pub cli_session_id: Option<String>,
    /// Text to type into the terminal after its first output.
    pub typed_input: Option<String>,
}

/// Where to look for the session that a CLI start creates.
#[derive(Debug, Clone, Copy)]
pub struct TrackingRequest<'a> {
    pub working_folder: &'a Path,
    /// The complete environment of the start, which says where the CLI keeps
    /// its sessions.
    pub environment: &'a [(String, String)],
}

/// The result of following a start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackedSession {
    Found(String),
    /// The session could not be told apart from others; the text says why.
    Unmatched(String),
}
