// SPDX-License-Identifier: AGPL-3.0-or-later

use audit_application::common::errors::AuditError;
use audit_application::queries::list_actions::ListActionsHandler;
use kernel::AgentId;
use serde_json::{Map, Value};
use wire::{ErrorCode, Request, Response};

use crate::mappings::action_entry_json;

/// The one request of this module. It sits in the `agent.` namespace of the
/// contract, so the host matches the whole type, not a prefix.
pub const ACTIONS_REQUEST: &str = "agent.actions";

/// The handlers of the audit requests.
#[derive(Clone)]
pub struct AuditHandlers {
    list: ListActionsHandler,
}

impl AuditHandlers {
    #[must_use]
    pub fn new(list: ListActionsHandler) -> Self {
        Self { list }
    }

    /// Answers `agent.actions` with the entries of one agent, oldest first,
    /// also when the agent no longer exists. A failed disk is named in the
    /// second part, so the host can raise `storage.error`.
    #[must_use]
    pub fn handle(&self, request: &Request) -> (Response, Option<String>) {
        let id = request.id;
        let agent_id = match read_agent_id(&request.fields) {
            Ok(agent_id) => agent_id,
            Err(message) => return (invalid(id, &message), None),
        };
        match self.list.handle(agent_id) {
            Ok(entries) => {
                let entries = entries.iter().map(action_entry_json).collect();
                (Response::success(id, Value::Array(entries)), None)
            }
            Err(error) => {
                let disk_failure = match &error {
                    AuditError::Storage {
                        message,
                        is_disk_failure: true,
                    } => Some(message.clone()),
                    _ => None,
                };
                (
                    Response::failure(id, ErrorCode::Internal, error.to_string()),
                    disk_failure,
                )
            }
        }
    }
}

/// Reads `agent_id`; every failure names the field.
fn read_agent_id(fields: &Map<String, Value>) -> Result<AgentId, String> {
    let text = match fields.get("agent_id") {
        None | Some(Value::Null) => return Err("agent_id is required".to_owned()),
        Some(Value::String(text)) => text,
        Some(_) => return Err("agent_id must be text".to_owned()),
    };
    text.parse()
        .map_err(|_| format!("agent_id is not a valid identifier: {text}"))
}

fn invalid(id: u32, message: &str) -> Response {
    Response::failure(id, ErrorCode::Invalid, message)
}

#[cfg(test)]
#[path = "audit_handlers.tests.rs"]
mod tests;
