// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;
use serde_json::{Map, Value, json};
use wire::{Event, Response};

const SECRET_CHANGED: &str = "secret.changed";

/// What the host records in the action record for one request. The text never
/// holds a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalEntry {
    /// A read of an agent, accepted or refused.
    Read {
        agent: AgentId,
        text: String,
        failed: bool,
    },
    /// A change of the person: the entry's agent is the one a grant names.
    Change {
        target: Option<AgentId>,
        text: String,
    },
}

/// A `secret.changed` event (contract): names only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeEvent {
    pub cause: &'static str,
    pub name: Option<String>,
    pub agent_id: Option<AgentId>,
}

/// The answer to one request and what the host does besides: record it and
/// tell the clients.
#[derive(Debug)]
pub struct Handled {
    pub response: Response,
    pub entry: Option<JournalEntry>,
    pub event: Option<ChangeEvent>,
}

impl Handled {
    pub(crate) fn answer(response: Response) -> Self {
        Self {
            response,
            entry: None,
            event: None,
        }
    }
}

#[must_use]
pub fn secret_changed_event(change: &ChangeEvent) -> Event {
    let mut fields = Map::new();
    fields.insert("cause".to_owned(), json!(change.cause));
    if let Some(name) = &change.name {
        fields.insert("name".to_owned(), Value::String(name.clone()));
    }
    if let Some(agent_id) = change.agent_id {
        fields.insert("agent_id".to_owned(), json!(agent_id.to_string()));
    }
    Event::new(SECRET_CHANGED, fields)
}
