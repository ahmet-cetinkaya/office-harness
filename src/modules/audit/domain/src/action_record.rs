// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::{Actor, AgentId, Timestamp};

use crate::{ActionKind, ActionResult};

/// One action, before the store gives it an id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRecord {
    pub actor: Actor,
    /// Kept after the agent is deleted, so the history stays readable.
    /// `None` is an office-level entry, such as an office hold.
    pub target_agent_id: Option<AgentId>,
    pub action: ActionKind,
    pub result: ActionResult,
    /// Why the action happened, when the action alone does not say it.
    pub detail: Option<String>,
    /// The schedule's name when the actor is a schedule, as it was when the
    /// action happened (0025 D7); it survives a rename or a delete.
    pub actor_label: Option<String>,
    pub at: Timestamp,
}

/// A stored action record entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionEntry {
    pub id: i64,
    pub record: ActionRecord,
}
