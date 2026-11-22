// SPDX-License-Identifier: AGPL-3.0-or-later

//! The answer of `agent.subtree` (0014): the state of the caller's
//! descendants. Each entry is built field by field, so nothing else of an
//! agent, and never its terminal content, reaches the reading agent (FR-003).

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::time::Duration;

use agent_domain::entities::Agent;
use agent_domain::value_objects::TaskText;
use hierarchy_domain::{Hold, Role, TreeRow, descendants_in_tree_order, hold_refusal};
use kernel::{AgentId, Timestamp};
use serde_json::{Value, json};

/// The most characters of the task text's first line an entry carries (the
/// task line of 0011/FR-001).
const TASK_LINE_MAX_CHARS: usize = 80;

const ELLIPSIS: char = '…';

/// The last C0 control character; U+0000 to here map to the control pictures.
const C0_END: u32 = 0x1f;
const CONTROL_PICTURES_START: u32 = 0x2400;
const DEL: u32 = 0x7f;
const DEL_PICTURE: char = '␡';
const C1_START: u32 = 0x80;
const C1_END: u32 = 0x9f;

/// Bidirectional format characters: they reorder the text around them, so a
/// reader could take the words in another order than they were written.
const BIDI_EMBEDDINGS_START: u32 = 0x202a;
const BIDI_EMBEDDINGS_END: u32 = 0x202e;
const BIDI_ISOLATES_START: u32 = 0x2066;
const BIDI_ISOLATES_END: u32 = 0x2069;

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

/// The text with every control and bidirectional format character shown as
/// a visible symbol, as the desktop's task line shows it, so nothing in it
/// acts on the reader's terminal or reorders what it reads (0011/FR-006).
#[must_use]
pub fn visible_text(text: &str) -> String {
    let mut visible = String::with_capacity(text.len());
    for character in text.chars() {
        push_visible(&mut visible, character);
    }
    visible
}

fn push_visible(visible: &mut String, character: char) {
    let code = u32::from(character);
    if code <= C0_END {
        let picture = char::from_u32(CONTROL_PICTURES_START + code);
        visible.push(picture.unwrap_or(char::REPLACEMENT_CHARACTER));
    } else if code == DEL {
        visible.push(DEL_PICTURE);
    } else if is_escaped(code) {
        // Writing into a String cannot fail.
        let _ = write!(visible, "\\u{code:04X}");
    } else {
        visible.push(character);
    }
}

/// C1 controls and the bidirectional format characters, which have no
/// picture of their own and are written out as their code.
fn is_escaped(code: u32) -> bool {
    (C1_START..=C1_END).contains(&code)
        || (BIDI_EMBEDDINGS_START..=BIDI_EMBEDDINGS_END).contains(&code)
        || (BIDI_ISOLATES_START..=BIDI_ISOLATES_END).contains(&code)
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
