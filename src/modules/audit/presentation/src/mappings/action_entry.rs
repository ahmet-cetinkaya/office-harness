// SPDX-License-Identifier: AGPL-3.0-or-later

use audit_domain::ActionEntry;
use kernel::Actor;
use serde_json::{Map, Value, json};
use wire::Event;

const ACTION_RECORDED_EVENT: &str = "action.recorded";

/// The wire form of one entry: who, what, which agent, how it ended, when.
#[must_use]
pub fn action_entry_json(entry: &ActionEntry) -> Value {
    let record = &entry.record;
    json!({
        "id": entry.id,
        "actor": actor_text(record.actor),
        "agent_id": record.target_agent_id.map(|agent_id| agent_id.to_string()),
        "action": record.action.as_str(),
        "result": record.result.as_text(),
        "at": record.at.as_unix_millis(),
    })
}

#[must_use]
pub fn action_recorded_event(entry: &ActionEntry) -> Event {
    let mut fields = Map::new();
    fields.insert("entry".to_owned(), action_entry_json(entry));
    Event::new(ACTION_RECORDED_EVENT, fields)
}

fn actor_text(actor: Actor) -> String {
    match actor {
        Actor::User => "user".to_owned(),
        Actor::Office => "office".to_owned(),
        Actor::Agent(agent_id) => format!("agent:{agent_id}"),
    }
}

#[cfg(test)]
#[path = "action_entry.tests.rs"]
mod tests;
