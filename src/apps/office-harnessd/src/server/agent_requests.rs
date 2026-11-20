// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, PoisonError};

use agent_presentation::{AgentHandlers, Handled, REQUEST_PREFIXES};
use audit_presentation::{ACTIONS_REQUEST, AuditHandlers};
use hierarchy_application::TreeQuery;
use hierarchy_application::queries::tree::RowPlace;
use kernel::AgentId;
use serde_json::Value;
use terminal_application::common::interfaces::TerminalRuntime;
use tokio::task::spawn_blocking;
use wire::{ErrorCode, Request, Response};

use super::event_hub::EventHub;
use super::hold_requests::HoldGateway;
use crate::bootstrap::StorageAlerts;
use crate::bridges::{
    NeedsYouBridge, StatusOrder, StoppingFlag, TerminalEvents, take_recorded_events,
};

/// The requests that change an agent's status or its terminal. They run one
/// at a time, and apart from the status bridge, so that a delete never
/// overtakes a slow start, and a stop never races an exit.
const DELETE_REQUEST: &str = "agent.delete";

const ORDERED_REQUESTS: [&str; 5] = [
    "agent.start",
    "agent.stop",
    DELETE_REQUEST,
    "agent.new_task",
    "session.continue",
];

/// The requests that start a child, which the server refuses once it stops.
const STARTING_REQUESTS: [&str; 3] = ["agent.start", "agent.new_task", "session.continue"];

const STOPPING_MESSAGE: &str = "the server is stopping and starts no agent";

/// The requests whose answers carry agent rows; `project.*`, `session.*`
/// and `settings.*` answers carry other ids and get no place fields.
const AGENT_PREFIX: &str = "agent.";

/// Runs the `agent.*`, `project.*`, `session.*`, and `settings.*` requests. The use cases
/// block, so each call runs on a blocking thread. The events that a request
/// causes go to every client, and a failed disk raises `storage.error`.
#[derive(Clone)]
pub struct AgentGateway {
    agents: Arc<AgentHandlers>,
    audit: AuditHandlers,
    events: EventHub,
    storage_alerts: StorageAlerts,
    status_order: StatusOrder,
    stopping: StoppingFlag,
    terminals: TerminalEvents,
    runtime: Arc<dyn TerminalRuntime>,
    hierarchy: Arc<TreeQuery>,
    holds: HoldGateway,
    needs_you: NeedsYouBridge,
}

impl AgentGateway {
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "the gateway joins the agent, audit, terminal, hierarchy, and access services"
    )]
    pub fn new(
        agents: Arc<AgentHandlers>,
        audit: AuditHandlers,
        events: EventHub,
        storage_alerts: StorageAlerts,
        status_order: StatusOrder,
        stopping: StoppingFlag,
        terminals: TerminalEvents,
        runtime: Arc<dyn TerminalRuntime>,
        hierarchy: Arc<TreeQuery>,
        holds: HoldGateway,
        needs_you: NeedsYouBridge,
    ) -> Self {
        Self {
            agents,
            audit,
            events,
            storage_alerts,
            status_order,
            stopping,
            terminals,
            runtime,
            hierarchy,
            holds,
            needs_you,
        }
    }

    /// True for the requests that this gateway answers: the whole of
    /// `agent.`, `project.`, `session.`, and `settings.`.
    #[must_use]
    pub fn accepts(kind: &str) -> bool {
        REQUEST_PREFIXES
            .iter()
            .any(|prefix| kind.starts_with(prefix))
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
}

impl AgentGateway {
    /// The answer, and the events it causes, of one request on the
    /// blocking pool it always runs on. The agent-call gateway forwards a
    /// checked request here, so both entrances share one handler and one
    /// ordering.
    pub(crate) fn run(&self, request: &Request) -> Response {
        if request.kind == ACTIONS_REQUEST {
            let (response, disk_failure) = self.audit.handle(request);
            self.raise(disk_failure.as_deref());
            return response;
        }
        let _turn = ORDERED_REQUESTS.contains(&request.kind.as_str()).then(|| {
            self.status_order
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
        });
        if refused_while_stopping(&request.kind, self.stopping.load(Ordering::SeqCst)) {
            return Response::failure(request.id, ErrorCode::Refused, STOPPING_MESSAGE);
        }
        // Drop what an earlier request on this thread left behind.
        drop(take_recorded_events());
        let Handled {
            mut response,
            events,
            disk_failure,
        } = self.agents.handle(request);
        self.raise(disk_failure.as_deref());
        if request.kind == DELETE_REQUEST {
            // A deleted agent leaves no hold behind (0007 D10); a subtree
            // delete takes the same path. It runs on failure too, because a
            // subtree delete that fails part way keeps the agents it already
            // deleted, and pruning a missing agent's hold is idempotent.
            self.holds.forget_deleted_agents();
            // The same for the Needs you items (0008 D2).
            self.needs_you.forget_deleted_agents();
        }
        self.add_last_lines(&mut response);
        if request.kind.starts_with(AGENT_PREFIX) {
            self.add_places(&mut response);
        }
        for event in events.iter().chain(&take_recorded_events()) {
            self.events.publish(event);
        }
        response
    }

    /// Fills `last_line` into every agent object the response carries, so
    /// `agent.list` shows the text the panel shows. The live updates arrive
    /// as `agent.line` events, so the events of this response need nothing.
    fn add_last_lines(&self, response: &mut Response) {
        let Response::Success { result, .. } = response else {
            return;
        };
        fill_last_lines(result, &|agent_id| {
            self.runtime
                .current_run(agent_id)
                .and_then(|run| self.terminals.last_line(agent_id, run))
        });
    }

    /// Fills `role` and `team` into every agent object the response
    /// carries, read from the hierarchy at answer time, so a rename or a
    /// placement shows in the next answer (hierarchy-protocol.md: `team` is
    /// the team's display name, never a key). A failed read raises the
    /// storage alert and leaves both fields null.
    fn add_places(&self, response: &mut Response) {
        let Response::Success { result, .. } = response else {
            return;
        };
        let places = self.hierarchy.row_places().unwrap_or_else(|error| {
            self.raise(Some(&error.to_string()));
            HashMap::new()
        });
        fill_places(result, &places);
    }

    fn raise(&self, disk_failure: Option<&str>) {
        if let Some(message) = disk_failure {
            self.storage_alerts.report_disk_failure(message);
        }
    }
}

/// Writes `last_line` into every agent object of a response result: the list
/// is an array of agents, every other agent response is one agent.
fn fill_last_lines(result: &mut Value, last_line: &dyn Fn(AgentId) -> Option<String>) {
    for agent in agent_objects(result) {
        let Some(id) = agent.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Ok(agent_id) = AgentId::from_str(id) else {
            continue;
        };
        let line = last_line(agent_id).map_or(Value::Null, Value::String);
        if let Some(agent) = agent.as_object_mut() {
            agent.insert("last_line".to_owned(), line);
        }
    }
}

/// Writes `role` and `team` into every agent object of a response result;
/// an agent outside the hierarchy gets null for both.
fn fill_places(result: &mut Value, places: &HashMap<AgentId, RowPlace>) {
    for agent in agent_objects(result) {
        let place = agent
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| AgentId::from_str(id).ok())
            .and_then(|agent_id| places.get(&agent_id));
        let role = place
            .and_then(|place| place.role)
            .map_or(Value::Null, |role| Value::from(role.as_str()));
        let team = place
            .and_then(|place| place.team_name.clone())
            .map_or(Value::Null, Value::String);
        if let Some(agent) = agent.as_object_mut() {
            agent.insert("role".to_owned(), role);
            agent.insert("team".to_owned(), team);
        }
    }
}

/// The agent objects of a response result: the list is an array of
/// agents, every other agent response is one agent.
fn agent_objects(result: &mut Value) -> Vec<&mut Value> {
    match result {
        Value::Array(agents) => agents.iter_mut().collect(),
        agent @ Value::Object(_) => vec![agent],
        _ => Vec::new(),
    }
}

/// True for a start request once the server is stopping. Checked after the
/// status lock is taken, which the interrupt holds while it sets the flag.
fn refused_while_stopping(kind: &str, stopping: bool) -> bool {
    stopping && STARTING_REQUESTS.contains(&kind)
}

#[cfg(test)]
#[path = "agent_requests.tests.rs"]
mod tests;
