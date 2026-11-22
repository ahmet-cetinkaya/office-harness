// SPDX-License-Identifier: AGPL-3.0-or-later

//! The answer of `agent.subtree` (0014): the state of the caller's
//! descendants. Each entry is built field by field, so nothing else of an
//! agent, and never its terminal content, reaches the reading agent (FR-003).

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use agent_domain::entities::Agent;
use agent_domain::value_objects::TaskText;
use hierarchy_domain::{
    Hold, Role, TreeRow, descendants_in_tree_order, hold_refusal, visible_text,
};
use kernel::{AgentId, Timestamp};
use serde_json::{Value, json};

/// The most characters of the task text's first line an entry carries (the
/// task line of 0011/FR-001).
const TASK_LINE_MAX_CHARS: usize = 80;

const ELLIPSIS: char = '…';

/// What one read sees: the tree, the agents, the agents with an item in
/// Needs you, and the holds, each taken once.
pub struct SubtreeState<'state> {
    pub rows: &'state [TreeRow],
    pub agents: &'state [Agent],
    pub waiting: &'state HashSet<AgentId>,
    pub holds: &'state [Hold],
}

/// One entry per descendant of `caller`, each right after its parent
/// (FR-001, FR-002). `last_output_at` answers for a running agent only. A
/// descendant that the tree names but the agents no longer hold (a delete
/// between the two reads) is left out.
#[must_use]
pub fn subtree_entries(
    state: &SubtreeState<'_>,
    caller: AgentId,
    last_output_at: &dyn Fn(AgentId) -> Option<Timestamp>,
) -> Vec<Value> {
    let agents: HashMap<AgentId, &Agent> =
        state.agents.iter().map(|agent| (agent.id, agent)).collect();
    descendants_in_tree_order(state.rows, caller)
        .into_iter()
        .filter_map(|row| {
            let agent = agents.get(&row.agent_id)?;
            Some(entry(state, row, agent, last_output_at(agent.id)))
        })
        .collect()
}

fn entry(
    state: &SubtreeState<'_>,
    row: &TreeRow,
    agent: &Agent,
    last_output_at: Option<Timestamp>,
) -> Value {
    json!({
        "agent_id": agent.id.to_string(),
        "name": visible_text(agent.name.as_str()),
        "parent_agent_id": row.parent_agent_id.map(|parent| parent.to_string()),
        "role": row.role.map(Role::as_str),
        "status": agent.status.as_str(),
        "last_output_at": last_output_at.map(Timestamp::as_unix_millis),
        "needs_you": state.waiting.contains(&agent.id),
        // The same coverage that refuses an agent-made action (0007/FR-004).
        "held": hold_refusal(state.holds, state.rows, agent.id, None).is_some(),
        "task": agent.task.as_ref().map_or(Value::Null, task_json),
    })
}

/// `{line, set_at}`: the task line and the time of the task text, never the
/// full text (FR-003).
fn task_json(task: &TaskText) -> Value {
    json!({
        "line": task_line(task.text()),
        "set_at": task.set_at().as_unix_millis(),
    })
}

/// The first line of the text, at most 80 characters, in visible form, with
/// an ellipsis when anything of the text is left out (0011/FR-001).
#[must_use]
pub fn task_line(text: &str) -> String {
    let first_line = text.split(['\r', '\n']).next().unwrap_or_default();
    let has_more_lines = !text[first_line.len()..].trim().is_empty();
    let is_too_long = first_line.chars().count() > TASK_LINE_MAX_CHARS;
    let kept: String = first_line.chars().take(TASK_LINE_MAX_CHARS).collect();
    let mut line = visible_text(&kept);
    if has_more_lines || is_too_long {
        line.push(ELLIPSIS);
    }
    line
}

/// The wall time `quiet` before `now`: when a running child last wrote.
#[must_use]
pub fn time_before(now: Timestamp, quiet: Duration) -> Timestamp {
    let quiet_millis = i64::try_from(quiet.as_millis()).unwrap_or(i64::MAX);
    Timestamp::from_unix_millis(now.as_unix_millis().saturating_sub(quiet_millis))
}

#[cfg(test)]
#[path = "subtree_status.tests.rs"]
mod tests;
