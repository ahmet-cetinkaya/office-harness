// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, OnceLock, PoisonError};

use agent_application::common::actor_scope::ActorScope;
use agent_application::common::interfaces::Clock;
use agent_application::common::session_launcher::SessionLauncher;
use agent_application::common::sign_in_scope::SignInScope;
use agent_domain::value_objects::SessionId;
use agent_presentation::{AgentHandlers, Handled, REQUEST_PREFIXES};
use audit_presentation::{ACTIONS_REQUEST, AuditHandlers};
use hierarchy_application::TreeQuery;
use hierarchy_application::queries::tree::RowPlace;
use hierarchy_domain::Hold;
use hierarchy_presentation::{
    CAUSE_AGENT_PLACED, CAUSE_AGENT_UNPLACED, CAUSE_SUBTREE_DELETED, CAUSE_SUBTREE_STOPPED,
    tree_event,
};
use kernel::AgentId;
use notification_presentation::NeedsYouUseCases;
use serde_json::Value;
use terminal_application::common::interfaces::TerminalRuntime;
use tokio::task::spawn_blocking;
use wire::{ErrorCode, Request, Response};

use super::agent_call::recorded_and_published;
use super::event_hub::EventHub;
use super::hold_requests::HoldGateway;
use super::subtree_status::{SubtreeState, subtree_entries, time_before};
use crate::bootstrap::StorageAlerts;
use crate::bridges::{
    END_CHECK_REQUEST, ENDING_MESSAGE, NeedsYouBridge, ScheduleBridge, StatusOrder, StoppingFlag,
    SystemClock, TaskOrder, TempHooks, TempSnapshot, TerminalEvents, take_recorded_events,
    temp_changed_event,
};
use crate::sweep::SweepGate;

/// The requests that change an agent's status or its terminal. They run one
/// at a time, and apart from the status bridge, so that a delete never
/// overtakes a slow start, and a stop never races an exit.
const DELETE_REQUEST: &str = "agent.delete";

const CREATE_REQUEST: &str = "agent.create";
const STOP_REQUEST: &str = "agent.stop";

/// The requests that change a task text, which publish `agent.task` after
/// they store; they hold the task order from the store to the publish.
const TASK_REQUESTS: [&str; 2] = ["agent.task.set", "agent.new_task"];

const ORDERED_REQUESTS: [&str; 9] = [
    END_CHECK_REQUEST,
    "agent.start",
    "agent.stop",
    DELETE_REQUEST,
    "agent.new_task",
    "session.continue",
    "agent.resume",
    "agent.sleep",
    "agent.wake",
];

/// The requests that start a child, which the server refuses once it stops.
const STARTING_REQUESTS: [&str; 5] = [
    "agent.start",
    "agent.new_task",
    CONTINUE_REQUEST,
    "agent.resume",
    "agent.wake",
];

/// The start request that names a session instead of an agent.
const CONTINUE_REQUEST: &str = "session.continue";

/// Requests the server sends itself never reach a client, so their id is
/// never matched with an answer.
const OWN_REQUEST_ID: u32 = 0;

const START_REQUEST: &str = "agent.start";

const NO_TEMPS: &str = "temps are not available";

pub(super) const STOPPING_MESSAGE: &str = "the server is stopping and starts no agent";

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
    task_order: TaskOrder,
    stopping: StoppingFlag,
    terminals: TerminalEvents,
    runtime: Arc<dyn TerminalRuntime>,
    hierarchy: Arc<TreeQuery>,
    holds: HoldGateway,
    needs_you: NeedsYouBridge,
    /// The sweep of 0017 that the starting requests wait for.
    sweep_gate: SweepGate,
    /// Runs the sign-in check of a start before the status order (0019).
    sign_in: Option<SessionLauncher>,
    /// What the gateway does around a request because of temps (0020).
    temps: Option<Arc<TempHooks>>,
    /// The schedule bridge, which the server connects after it built the
    /// gateway that the bridge wakes agents through (0025 D13).
    schedules: Arc<OnceLock<ScheduleBridge>>,
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
        task_order: TaskOrder,
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
            task_order,
            stopping,
            terminals,
            runtime,
            hierarchy,
            holds,
            needs_you,
            sweep_gate: SweepGate::default(),
            sign_in: None,
            temps: None,
            schedules: Arc::default(),
        }
    }

    /// Connects the schedule bridge, which switches the schedules of a
    /// removed agent off after each delete (0025/FR-010). Every clone of the
    /// gateway shares the connection.
    pub fn connect_schedules(&self, schedules: ScheduleBridge) {
        // The server connects once; a second call has nothing to change.
        let _ = self.schedules.set(schedules);
    }

    /// Attaches the temp hooks: the refusal of a start while a temp ends, the
    /// end check, the start and objective of a new temp, and the finished
    /// entry of a temp the person deletes (0020).
    #[must_use]
    pub fn with_temps(mut self, temps: TempHooks) -> Self {
        self.temps = Some(Arc::new(temps));
        self
    }

    /// Makes the requests that start a child wait until the sweep of 0017 is
    /// over (FR-007).
    #[must_use]
    pub fn with_sweep_gate(mut self, sweep_gate: SweepGate) -> Self {
        self.sweep_gate = sweep_gate;
        self
    }

    /// Runs each start's sign-in check before the start takes its turn in
    /// the status order, so no other request waits for a status command
    /// (0019/FR-009, D13).
    #[must_use]
    pub fn with_sign_in_checks(mut self, launcher: SessionLauncher) -> Self {
        self.sign_in = Some(launcher);
        self
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
        // Before the status order: a waiting start must not hold it. A temp
        // create starts its CLI at once, so it waits too (0017/FR-007).
        let starts_cli =
            STARTING_REQUESTS.contains(&request.kind.as_str()) || is_temp_create(&request);
        if starts_cli && self.sweep_gate.wait_until_done().await.is_err() {
            return Response::failure(id, ErrorCode::Refused, STOPPING_MESSAGE);
        }
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
        // Held to the end of the request, which includes the start of a new
        // temp, so the watcher never judges the temp before its start.
        let _creating = self
            .temps
            .as_ref()
            .filter(|_| is_temp_create(request))
            .map(|temps| temps.endings.begin_create());
        // Held to the end of the request, which is where the start reads it.
        let _sign_in = self.check_sign_in_ahead(request);
        let _turn = ORDERED_REQUESTS.contains(&request.kind.as_str()).then(|| {
            self.status_order
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
        });
        if let Some(response) = self.temp_answer(request) {
            return response;
        }
        if refused_while_stopping(&request.kind, self.stopping.load(Ordering::SeqCst)) {
            return Response::failure(request.id, ErrorCode::Refused, STOPPING_MESSAGE);
        }
        // Taken after the status order and held to the end of the function,
        // which is after the events are published.
        let _task_turn = TASK_REQUESTS.contains(&request.kind.as_str()).then(|| {
            self.task_order
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
        });
        // Drop what an earlier request on this thread left behind.
        drop(take_recorded_events());
        let had_place = self.has_place(request);
        let deleting = self.temps_before_delete(request);
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
            // And the schedules that sent to a removed agent (0025 D13).
            if let Some(schedules) = self.schedules.get() {
                schedules.forget_removed_targets();
            }
        }
        self.add_last_lines(&mut response);
        if request.kind.starts_with(AGENT_PREFIX) {
            self.add_places(&mut response);
        }
        // The agent's own events go first, so a client knows a new agent
        // before the tree names it.
        let tree_change = tree_cause(request, &response, had_place).map(tree_event);
        for event in events
            .iter()
            .chain(&tree_change)
            .chain(&take_recorded_events())
        {
            self.events.publish(event);
        }
        self.after_temp_request(request, &response, &deleting);
        response
    }

    /// The answer of a request that the temp rules decide before it runs: the
    /// end check, or the start of a temp that ends (0020 D7).
    fn temp_answer(&self, request: &Request) -> Option<Response> {
        if request.kind == END_CHECK_REQUEST {
            return Some(self.temps.as_ref().map_or_else(
                || Response::failure(request.id, ErrorCode::NotFound, NO_TEMPS),
                |temps| temps.endings.end_check(request),
            ));
        }
        let ending = self
            .temps
            .as_ref()
            .is_some_and(|temps| temps.endings.refuses(request));
        ending.then(|| Response::failure(request.id, ErrorCode::Conflict, ENDING_MESSAGE))
    }

    /// The temps that a delete by the person removes, read before it runs.
    fn temps_before_delete(&self, request: &Request) -> Vec<TempSnapshot> {
        let Some(temps) = self
            .temps
            .as_ref()
            .filter(|_| request.kind == DELETE_REQUEST)
        else {
            return Vec::new();
        };
        match self.hierarchy.store_snapshot() {
            Ok(rows) => temps.before_delete(&rows, request),
            Err(error) => {
                self.raise(Some(&error.to_string()));
                Vec::new()
            }
        }
    }

    /// The work after a request that concerns temps: the finished entries of
    /// deleted temps, and the start and objective of a new temp.
    fn after_temp_request(
        &self,
        request: &Request,
        response: &Response,
        deleting: &[TempSnapshot],
    ) {
        let Some(temps) = &self.temps else {
            return;
        };
        if request.kind == DELETE_REQUEST {
            temps.after_delete(deleting, request, response);
        } else if request.kind == CREATE_REQUEST {
            self.start_new_temp(temps, response);
        }
    }

    /// D5: starts the temp as its creator, then queues its objective at the
    /// head of its queue. A start that fails leaves the temp failed, which
    /// the watcher ends.
    fn start_new_temp(&self, temps: &TempHooks, response: &Response) {
        let Response::Success { result, .. } = response else {
            return;
        };
        let objective = result.pointer("/temp/objective").and_then(Value::as_str);
        let Some((agent_id, objective)) = result
            .get("id")
            .and_then(Value::as_str)
            .and_then(AgentId::from_uuid_text)
            .zip(objective)
        else {
            return;
        };
        let mut fields = serde_json::Map::new();
        fields.insert("agent_id".to_owned(), Value::String(agent_id.to_string()));
        drop(self.run(&Request {
            id: OWN_REQUEST_ID,
            kind: START_REQUEST.to_owned(),
            fields,
        }));
        // A refusal is journaled by the handler itself.
        let _ = recorded_and_published(&self.events, || {
            temps
                .directives
                .queue_objective(agent_id, objective, ActorScope::current())
        });
        self.events.publish(&temp_changed_event());
    }

    /// The sign-in check of the agent that a start request names, or of the
    /// session's agent for a continue. A request it cannot read gets no
    /// check here; the start then checks in place.
    fn check_sign_in_ahead(&self, request: &Request) -> Option<SignInScope> {
        let launcher = self.sign_in.as_ref()?;
        // A stopping server refuses the start anyway.
        let is_stopping = self.stopping.load(Ordering::SeqCst);
        if is_stopping || !STARTING_REQUESTS.contains(&request.kind.as_str()) {
            return None;
        }
        let text = |name: &str| request.fields.get(name).and_then(Value::as_str);
        if request.kind == CONTINUE_REQUEST {
            let session_id = SessionId::from_str(text("session_id")?).ok()?;
            return launcher.check_sign_in_ahead_of_session(session_id);
        }
        let agent_id = AgentId::from_str(text("agent_id")?).ok()?;
        launcher.check_sign_in_ahead(agent_id)
    }

    /// True when the agent that a delete names holds a place in the tree
    /// before the delete runs; after it, the place is gone. A failed read
    /// raises the storage alert and counts as no place.
    ///
    /// The read runs outside the tree exclusion, which the delete takes
    /// inside the hierarchy module. A place change by another client between
    /// the read and the delete can pick the wrong cause or skip the event.
    /// That costs one missing or one extra re-read of the tree, which is
    /// harmless, and a bridge to hold the exclusion across both steps would
    /// cost more than the re-read it saves.
    fn has_place(&self, request: &Request) -> bool {
        if request.kind != DELETE_REQUEST {
            return false;
        }
        let Some(agent_id) = request
            .fields
            .get("agent_id")
            .and_then(Value::as_str)
            .and_then(|id| AgentId::from_str(id).ok())
        else {
            return false;
        };
        match self.hierarchy.row_places() {
            Ok(places) => places
                .get(&agent_id)
                .is_some_and(|place| place.role.is_some()),
            Err(error) => {
                self.raise(Some(&error.to_string()));
                false
            }
        }
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

    /// The state of the caller's descendants for `agent.subtree` (0014). It
    /// reads stored state and each running session's quiet time, and takes
    /// neither the status order, nor the task order, nor the tree exclusion,
    /// so a read blocks no change of the office (spec A11, plan D10).
    ///
    /// # Errors
    /// The text of a failed read of the tree, the agents, or Needs you; the
    /// failure also raises the storage alert, as the other reads here do.
    pub(crate) fn subtree_status(&self, caller: AgentId, holds: &[Hold]) -> Result<Value, String> {
        let read_failed = |error: &dyn std::fmt::Display| {
            let message = error.to_string();
            self.raise(Some(&message));
            message
        };
        let rows = self
            .hierarchy
            .store_snapshot()
            .map_err(|error| read_failed(&error))?;
        let agents = self
            .agents
            .use_cases()
            .list_agents
            .handle()
            .map_err(|error| read_failed(&error))?;
        let waiting: HashSet<AgentId> = self
            .needs_you
            .list()
            .map_err(|error| read_failed(&error))?
            .iter()
            .map(|item| item.agent_id)
            .collect();
        let state = SubtreeState {
            rows: &rows,
            agents: &agents,
            waiting: &waiting,
            holds,
        };
        let now = SystemClock.now();
        let last_output_at = |agent_id| {
            self.runtime
                .quiet_for(agent_id)
                .map(|quiet| time_before(now, quiet))
        };
        Ok(Value::Array(subtree_entries(
            &state,
            caller,
            &last_output_at,
        )))
    }

    fn raise(&self, disk_failure: Option<&str>) {
        if let Some(message) = disk_failure {
            self.storage_alerts.report_disk_failure(message);
        }
    }
}

/// The `hierarchy.changed` cause of a successful agent request that
/// changes the tree: a create with a `role` places the agent, a delete
/// removes its place (or a whole subtree), a stop of a subtree changes the
/// states the tree shows. A failed request changed nothing, and neither
/// did a delete of an agent that had no place.
fn tree_cause(request: &Request, response: &Response, had_place: bool) -> Option<&'static str> {
    if !matches!(response, Response::Success { .. }) {
        return None;
    }
    let covers_subtree = request
        .fields
        .get("include_descendants")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    match request.kind.as_str() {
        CREATE_REQUEST => request
            .fields
            .get("role")
            .is_some_and(|role| !role.is_null())
            .then_some(CAUSE_AGENT_PLACED),
        DELETE_REQUEST if !had_place => None,
        DELETE_REQUEST if covers_subtree => Some(CAUSE_SUBTREE_DELETED),
        DELETE_REQUEST => Some(CAUSE_AGENT_UNPLACED),
        STOP_REQUEST => covers_subtree.then_some(CAUSE_SUBTREE_STOPPED),
        _ => None,
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
/// an agent outside the hierarchy gets null for both. An object without an
/// agent `id` is not an agent (the worktree answers of 0010) and stays as
/// it is.
pub(super) fn fill_places(result: &mut Value, places: &HashMap<AgentId, RowPlace>) {
    for agent in agent_objects(result) {
        let Some(agent_id) = agent
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| AgentId::from_str(id).ok())
        else {
            continue;
        };
        let place = places.get(&agent_id);
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

/// True for an `agent.create` that makes a temp.
fn is_temp_create(request: &Request) -> bool {
    request.kind == CREATE_REQUEST && request.fields.get("temp") == Some(&Value::Bool(true))
}

/// True for a start request once the server is stopping. Checked after the
/// status lock is taken, which the interrupt holds while it sets the flag.
fn refused_while_stopping(kind: &str, stopping: bool) -> bool {
    stopping && STARTING_REQUESTS.contains(&kind)
}

#[cfg(test)]
#[path = "agent_requests.tests.rs"]
mod tests;
