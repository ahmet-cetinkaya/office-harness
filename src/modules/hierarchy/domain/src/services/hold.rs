// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;
use std::fmt;

use kernel::{AgentId, Timestamp};

use super::tree_rules::TreeRow;

/// What a hold covers: the whole office, or one agent and its subtree (A2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldScope {
    Office,
    Agent(AgentId),
}

/// Who put the hold on: the user, or the office itself after the third limit
/// refusal (FR-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldSetter {
    User,
    Office,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hold {
    pub scope: HoldScope,
    pub set_by: HoldSetter,
    pub since: Timestamp,
    pub reason: Option<String>,
}

/// The reason a hold refuses an agent-made action. The Display text is the
/// answer the agent reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldRefusal {
    Office,
    Agent(AgentId),
}

impl fmt::Display for HoldRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Office => write!(formatter, "the office is on hold"),
            Self::Agent(agent_id) => write!(formatter, "the agent {agent_id} is on hold"),
        }
    }
}

/// The first hold that refuses an agent-made action (FR-002, FR-004): the
/// office hold, then an agent hold on the caller or one of its ancestors,
/// then one on the target or one of its ancestors. `target` is `None` when the
/// action has none; the caller is still checked.
///
/// Ancestors are read on `rows`, the tree at the moment of the request, so an
/// agent that joined a held subtree later is covered at once (A2).
#[must_use]
pub fn hold_refusal(
    holds: &[Hold],
    rows: &[TreeRow],
    caller: AgentId,
    target: Option<AgentId>,
) -> Option<HoldRefusal> {
    if holds.iter().any(|hold| hold.scope == HoldScope::Office) {
        return Some(HoldRefusal::Office);
    }
    let held_agents: HashSet<AgentId> = holds
        .iter()
        .filter_map(|hold| match hold.scope {
            HoldScope::Agent(agent_id) => Some(agent_id),
            HoldScope::Office => None,
        })
        .collect();
    std::iter::once(caller)
        .chain(target)
        .find_map(|agent_id| held_self_or_ancestor(&held_agents, rows, agent_id))
        .map(HoldRefusal::Agent)
}

/// The agent itself or the nearest ancestor that is held. The walk is capped
/// at the row count so a malformed snapshot with a cycle ends in a plain
/// no-hold instead of a hang.
fn held_self_or_ancestor(
    held_agents: &HashSet<AgentId>,
    rows: &[TreeRow],
    agent_id: AgentId,
) -> Option<AgentId> {
    let mut current = agent_id;
    for _ in 0..=rows.len() {
        if held_agents.contains(&current) {
            return Some(current);
        }
        current = rows
            .iter()
            .find(|row| row.agent_id == current)?
            .parent_agent_id?;
    }
    None
}

#[cfg(test)]
#[path = "hold.tests.rs"]
mod tests;
