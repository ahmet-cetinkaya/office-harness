// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;
use serde_json::{Map, Value, json};
use wire::Event;

use crate::bridges::{HookState, StatusSource};

/// The event that tells the clients how one agent's session reports its
/// status (0024 D9).
pub const HOOK_CHANGED_EVENT: &str = "hook.changed";

const fn source_name(source: StatusSource) -> &'static str {
    match source {
        StatusSource::Estimated => "estimated",
        StatusSource::Exact => "exact",
        StatusSource::Late => "late",
    }
}

/// The `hook` object of an agent: the status source with its reason, and
/// whether a permission ask is open or a compaction runs. `null` when the
/// agent has no running session.
#[must_use]
pub fn hook_json(state: Option<&HookState>) -> Value {
    let Some(state) = state else {
        return Value::Null;
    };
    json!({
        "source": source_name(state.source),
        "reason": state.reason,
        "ask_open": state.ask_open,
        "compacting": state.compacting,
    })
}

/// `hook.changed`: the agent and its new `hook` object.
#[must_use]
pub fn hook_changed_event(agent_id: AgentId, state: Option<&HookState>) -> Event {
    let mut fields = Map::new();
    fields.insert("agent_id".to_owned(), Value::String(agent_id.to_string()));
    fields.insert("hook".to_owned(), hook_json(state));
    Event::new(HOOK_CHANGED_EVENT, fields)
}

#[cfg(test)]
#[path = "hook_events.tests.rs"]
mod tests;
