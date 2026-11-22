// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::{Actor, AgentId, Timestamp};

/// The actions of this module that the action record keeps (FR-011).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentAction {
    Create,
    Start,
    Stop,
    Delete,
    NewTask,
    Continue,
    TeamCreate,
    TeamDelete,
    Directive,
    Refused,
    /// The user set, edited, or cleared the task text (0011).
    TaskText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentActionEntry {
    pub actor: Actor,
    pub agent_id: AgentId,
    pub action: AgentAction,
    /// The error text, or `None` when the action worked.
    pub failure: Option<String>,
    pub at: Timestamp,
}
