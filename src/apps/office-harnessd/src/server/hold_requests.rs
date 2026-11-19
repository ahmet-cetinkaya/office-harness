// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;
use std::sync::Arc;

use agent_application::common::interfaces::AgentStore;
use audit_application::common::errors::AuditError;
use audit_application::queries::list_office_actions::ListOfficeActionsHandler;
use audit_presentation::action_entry_json;
use hierarchy_application::common::errors::HierarchyApplicationError;
use hierarchy_application::{HoldService, HoldsView};
use hierarchy_domain::{Hold, HoldScope, HoldSetter};
use kernel::AgentId;
use serde_json::{Map, Value, json};
use tokio::task::spawn_blocking;
use wire::{ErrorCode, Event, Request, Response};

use super::event_hub::EventHub;
use crate::bootstrap::StorageAlerts;

const HOLD_SET: &str = "hold.set";
const HOLD_RELEASE: &str = "hold.release";
const HOLD_LIST: &str = "hold.list";
const OFFICE_ACTIONS: &str = "office.actions";
const HOLD_REQUESTS: [&str; 4] = [HOLD_SET, HOLD_RELEASE, HOLD_LIST, OFFICE_ACTIONS];

const HOLD_CHANGED_EVENT: &str = "hold.changed";

const AGENT_ID_FIELD: &str = "agent_id";

/// The wire form of one hold; the office hold has no `agent_id`, and a user
/// hold has no `reason` (hold-protocol contract).
#[must_use]
pub fn hold_json(hold: &Hold) -> Value {
    let mut fields = Map::new();
    if let HoldScope::Agent(agent_id) = hold.scope {
        fields.insert("agent_id".to_owned(), json!(agent_id.to_string()));
    }
    let set_by = match hold.set_by {
        HoldSetter::User => "user",
        HoldSetter::Office => "office",
    };
    fields.insert("set_by".to_owned(), json!(set_by));
    if let Some(reason) = &hold.reason {
        fields.insert("reason".to_owned(), json!(reason));
    }
    fields.insert("since".to_owned(), json!(hold.since.as_unix_millis()));
    Value::Object(fields)
}

fn holds_fields(view: &HoldsView) -> Map<String, Value> {
    let mut fields = Map::new();
    fields.insert(
        "office".to_owned(),
        view.office.as_ref().map_or(Value::Null, hold_json),
    );
    fields.insert(
        "agents".to_owned(),
        Value::Array(view.agents.iter().map(hold_json).collect()),
    );
    fields
}

/// `hold.changed`: the full hold state, which clients replace their copy
/// with.
#[must_use]
pub fn hold_changed_event(view: &HoldsView) -> Event {
    Event::new(HOLD_CHANGED_EVENT, holds_fields(view))
}

/// Runs `hold.set`, `hold.release`, `hold.list`, and `office.actions`, for
/// the user's connections only: the agent connection answers none of them.
/// The use cases block, so each call runs on a blocking thread, and every
/// change goes to all clients as `hold.changed`.
#[derive(Clone)]
pub struct HoldGateway {
    holds: Arc<HoldService>,
    agents: Arc<dyn AgentStore>,
    office_actions: ListOfficeActionsHandler,
    events: EventHub,
    alerts: StorageAlerts,
}

impl HoldGateway {
    #[must_use]
    pub fn new(
        holds: Arc<HoldService>,
        agents: Arc<dyn AgentStore>,
        office_actions: ListOfficeActionsHandler,
        events: EventHub,
        alerts: StorageAlerts,
    ) -> Self {
        Self {
            holds,
            agents,
            office_actions,
            events,
            alerts,
        }
    }

    #[must_use]
    pub fn accepts(kind: &str) -> bool {
        HOLD_REQUESTS.contains(&kind)
    }

    pub async fn handle(&self, request: Request) -> Response {
        let id = request.id;
        let gateway = self.clone();
        spawn_blocking(move || gateway.run(&request))
            .await
            .unwrap_or_else(|error| {
                Response::failure(
                    id,
                    ErrorCode::Internal,
                    format!("the request task failed: {error}"),
                )
            })
    }

    fn run(&self, request: &Request) -> Response {
        if let Err((code, message)) = refuse_unknown_fields(request) {
            return Response::failure(request.id, code, message);
        }
        match request.kind.as_str() {
            HOLD_LIST => {
                Response::success(request.id, Value::Object(holds_fields(&self.holds.list())))
            }
            HOLD_SET => self.change(request, |scope| self.holds.set_hold(scope)),
            HOLD_RELEASE => self.change(request, |scope| self.holds.release_hold(scope)),
            _ => self.list_office_actions(request.id),
        }
    }

    /// Reads the scope, runs the change, and tells every client the new
    /// state. The answer to `hold.set` carries the hold it made.
    fn change(
        &self,
        request: &Request,
        apply: impl FnOnce(HoldScope) -> Result<HoldsView, HierarchyApplicationError>,
    ) -> Response {
        let scope = match scope_of(request) {
            Ok(scope) => scope,
            Err((code, message)) => return Response::failure(request.id, code, message),
        };
        // Only a new hold needs a living agent: a hold left by a delete that
        // failed part way must still be releasable.
        if request.kind == HOLD_SET
            && let Err((code, message)) = self.ensure_agent_exists(scope)
        {
            return Response::failure(request.id, code, message);
        }
        match apply(scope) {
            Ok(view) => {
                self.events.publish(&hold_changed_event(&view));
                Response::success(request.id, result_of(request, scope, &view))
            }
            Err(error) => Response::failure(request.id, error_code(&error), message_of(&error)),
        }
    }

    /// Drops the holds of agents that were deleted and tells the clients
    /// when any went (0007 D10). The user's `agent.delete` calls this after
    /// it succeeded, for one agent or a whole subtree.
    pub fn forget_deleted_agents(&self) {
        let before = self.holds.list();
        // Only agent holds can outlive an agent.
        if before.agents.is_empty() {
            return;
        }
        let existing = match self.agents.list() {
            Ok(listed) => listed.iter().map(|agent| agent.id).collect::<HashSet<_>>(),
            Err(error) => return self.report(&format!("the holds were not pruned: {error}")),
        };
        if let Err(error) = self.holds.prune(&existing) {
            return self.report(&format!("the holds were not pruned: {error}"));
        }
        let after = self.holds.list();
        if after != before {
            self.events.publish(&hold_changed_event(&after));
        }
    }

    fn report(&self, message: &str) {
        self.alerts.report_disk_failure(message);
    }

    /// `not_found` for an agent scope whose agent does not exist.
    fn ensure_agent_exists(&self, scope: HoldScope) -> Result<(), (ErrorCode, String)> {
        let HoldScope::Agent(agent_id) = scope else {
            return Ok(());
        };
        match self.agents.find(agent_id) {
            Ok(Some(_)) => Ok(()),
            Ok(None) => Err((ErrorCode::NotFound, "no agent has this id".to_owned())),
            Err(error) => Err((ErrorCode::Internal, error.to_string())),
        }
    }

    fn list_office_actions(&self, id: u32) -> Response {
        match self.office_actions.handle() {
            Ok(entries) => Response::success(
                id,
                Value::Array(entries.iter().map(action_entry_json).collect()),
            ),
            Err(error) => {
                if let AuditError::Storage {
                    message,
                    is_disk_failure: true,
                } = &error
                {
                    self.alerts.report_disk_failure(message);
                }
                Response::failure(id, ErrorCode::Internal, error.to_string())
            }
        }
    }
}

/// The office when no `agent_id` is named; else the named agent.
fn scope_of(request: &Request) -> Result<HoldScope, (ErrorCode, String)> {
    let agent_id = match request.fields.get(AGENT_ID_FIELD) {
        None | Some(Value::Null) => return Ok(HoldScope::Office),
        Some(Value::String(text)) => AgentId::from_uuid_text(text)
            .ok_or_else(|| (ErrorCode::Invalid, "agent_id is not an agent id".to_owned()))?,
        Some(_) => {
            return Err((ErrorCode::Invalid, "agent_id must be text".to_owned()));
        }
    };
    Ok(HoldScope::Agent(agent_id))
}

/// `invalid` for a field the request does not take, so a misspelt
/// `agent_id` never falls back to the office scope.
fn refuse_unknown_fields(request: &Request) -> Result<(), (ErrorCode, String)> {
    let known: &[&str] = match request.kind.as_str() {
        HOLD_SET | HOLD_RELEASE => &[AGENT_ID_FIELD],
        _ => &[],
    };
    match request
        .fields
        .keys()
        .find(|field| !known.contains(&field.as_str()))
    {
        Some(field) => Err((
            ErrorCode::Invalid,
            format!("{field} is not a field of {}", request.kind),
        )),
        None => Ok(()),
    }
}

/// `{hold}` for `hold.set`, `{}` for `hold.release`.
fn result_of(request: &Request, scope: HoldScope, view: &HoldsView) -> Value {
    if request.kind != HOLD_SET {
        return json!({});
    }
    let made = match scope {
        HoldScope::Office => view.office.as_ref(),
        HoldScope::Agent(_) => view.agents.iter().find(|hold| hold.scope == scope),
    };
    json!({ "hold": made.map(hold_json) })
}

/// A conflict answers with its text alone, as the contract words it; the
/// error's own display adds a `conflict: ` prefix.
fn message_of(error: &HierarchyApplicationError) -> String {
    match error {
        HierarchyApplicationError::Conflict(text) => text.clone(),
        other => other.to_string(),
    }
}

fn error_code(error: &HierarchyApplicationError) -> ErrorCode {
    match error {
        HierarchyApplicationError::Conflict(_) | HierarchyApplicationError::ConflictInput(_) => {
            ErrorCode::Conflict
        }
        HierarchyApplicationError::NotFound(_) | HierarchyApplicationError::NotFoundInput(_) => {
            ErrorCode::NotFound
        }
        HierarchyApplicationError::Invalid(_) | HierarchyApplicationError::InvalidInput(_) => {
            ErrorCode::Invalid
        }
        HierarchyApplicationError::Refused(_) => ErrorCode::Refused,
        HierarchyApplicationError::Storage(_) => ErrorCode::Internal,
    }
}

#[cfg(test)]
#[path = "hold_requests.tests.rs"]
mod tests;
