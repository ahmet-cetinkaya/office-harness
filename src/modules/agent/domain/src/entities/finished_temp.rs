// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use kernel::{Actor, AgentId, Timestamp};

use crate::value_objects::FinishedTempId;

/// The last report a temp sent before it ended: the kind (`done`, `blocked`,
/// and so on, as the report named it) and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishedReport {
    pub kind: String,
    pub text: String,
}

/// What a temp left behind when it ended, kept until the person dismisses it
/// (0020 D11). The agent is gone, so the entry holds its name and objective
/// as plain text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishedTemp {
    pub id: FinishedTempId,
    pub agent_id: AgentId,
    pub name: String,
    pub objective: String,
    pub creator: Actor,
    /// The name of the agent it reported to, when it had one.
    pub parent_name: Option<String>,
    pub started_at: Timestamp,
    pub ended_at: Timestamp,
    pub reason: String,
    pub report: Option<FinishedReport>,
    /// The worktree folder the office kept because it held work.
    pub kept_folder: Option<PathBuf>,
    /// The branch the office kept because it held commits.
    pub kept_branch: Option<String>,
    /// Why the office could not remove the worktree, when it could not.
    pub removal_failure: Option<String>,
}
