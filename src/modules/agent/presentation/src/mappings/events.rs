// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::models::AdditionsView;
use agent_domain::entities::Agent;
use agent_domain::enums::AgentStatus;
use kernel::AgentId;
use serde_json::{Map, Value};
use wire::Event;

use super::{additions_json, agent_json};

/// `agent.status`. `detail` carries the text of a failure.
#[must_use]
pub fn agent_status_event(agent_id: AgentId, status: AgentStatus, detail: Option<&str>) -> Event {
    let mut fields = Map::new();
    fields.insert("agent_id".to_owned(), Value::String(agent_id.to_string()));
    fields.insert(
        "status".to_owned(),
        Value::String(status.as_str().to_owned()),
    );
    if let Some(detail) = detail {
        fields.insert("detail".to_owned(), Value::String(detail.to_owned()));
    }
    Event::new("agent.status", fields)
}

#[must_use]
pub fn agent_changed_event(agent: &Agent) -> Event {
    let mut fields = Map::new();
    fields.insert("agent".to_owned(), agent_json(agent));
    Event::new("agent.changed", fields)
}
/// `agent.line`: the agent's last output line changed.
#[must_use]
pub fn agent_line_event(agent_id: AgentId, line: &str) -> Event {
    let mut fields = Map::new();
    fields.insert("agent_id".to_owned(), Value::String(agent_id.to_string()));
    fields.insert("line".to_owned(), Value::String(line.to_owned()));
    Event::new("agent.line", fields)
}

#[must_use]
pub fn agent_deleted_event(agent_id: AgentId) -> Event {
    let mut fields = Map::new();
    fields.insert("agent_id".to_owned(), Value::String(agent_id.to_string()));
    Event::new("agent.deleted", fields)
}

/// `agent.additions.changed`: another client's open editor refreshes.
#[must_use]
pub fn additions_changed_event(agent_id: AgentId, view: &AdditionsView) -> Event {
    let mut fields = Map::new();
    fields.insert("agent_id".to_owned(), Value::String(agent_id.to_string()));
    fields.insert("additions".to_owned(), additions_json(view));
    Event::new("agent.additions.changed", fields)
}

#[cfg(test)]
#[path = "events.tests.rs"]
mod tests;
