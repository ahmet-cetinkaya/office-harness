// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use access_application::commands::authenticate_agent_call::AgentGrant;
use agent_application::common::actor_scope::ActorScope;
use axum::extract::ws::{Message, WebSocket};
use hierarchy_application::commands::send_directive::DirectiveOutcome;
use hierarchy_application::commands::send_report::{Report, ReportDelivery};
use hierarchy_application::common::errors::HierarchyApplicationError;
use hierarchy_domain::{
    AgentAction, HierarchyError, Hold, ReportKind, Role, TeamId, TreeRow, check_descendant,
    is_descendant_of,
};
use kernel::{Actor, AgentId};
use serde_json::{Value, json};
use tokio::sync::watch;
use tokio::task::spawn_blocking;
use wire::{CLOSE_UNAUTHORIZED, ErrorCode, Request, Response};

use super::connection::{CLOSE_GOING_AWAY, close_with, send_text};
use super::event_hub::EventHub;
use super::server_state::ServerState;
use crate::bridges::take_recorded_events;

/// The only request types an agent connection answers (0002, plan D3). Any
/// other type is refused, so no agent reaches `terminal.*`, `agent.delete`,
/// `team.*`, `hierarchy.*`, `server.*`, `settings.*`, `project.*`,
/// `template.*`, or the additions of an agent.
const ALLOWED_REQUESTS: [&str; 8] = [
    "agent.whoami",
    "agent.subtree",
    "agent.create",
    "agent.start",
    "agent.new_task",
    "agent.stop",
    "directive.send",
    "report.send",
];

/// What the connection of an agent says about a request it will not run.
const NOT_ALLOWED: &str = "an agent may only create, start, stop, give new tasks to, send directives to, and read the state of its own descendants, and send reports to its parent or the person";

/// The longest client text a refusal reason keeps: the type and a field key
/// travel into the log and the record, so the limit keeps one hostile frame
/// from flooding either.
const MAX_REFUSAL_TEXT_CHARS: usize = 64;

/// A field a request must never carry: the server fixes the actor from the
/// authenticated credential, so a client naming an actor is smuggling an
/// identity past the proof (0002 contract, agent caller).
const FORBIDDEN_FIELDS: [&str; 2] = ["actor", "caller"];

/// The fields `directive.send` takes, and no others.
const DIRECTIVE_FIELDS: [&str; 2] = ["target_agent_id", "text"];

/// The fields `report.send` takes, and no others: none of them names an
/// agent, so a report can reach only the sender's parent or the person
/// (0015/FR-002).
const REPORT_FIELDS: [&str; 3] = ["kind", "text", "for_person"];

/// Why the gateway refused a frame, as the action record keeps it.
enum Refusal {
    /// The type is not one an agent may ask for.
    NotAllowed,
    /// The type is allowed but not served yet.
    NotYet,
    /// The frame names an actor or caller, which the credential already is.
    SmuggledField(String),
}

/// Serves one authenticated agent connection: requests and answers, nothing
/// else. It has no events and no terminals. The connection ends when the
/// agent's session ends, so a revoked credential stops the connection and not
/// only new ones.
pub async fn serve_agent_connection(mut socket: WebSocket, state: ServerState, grant: AgentGrant) {
    let mut stopping = state.shutdown.clone();
    loop {
        tokio::select! {
            biased;
            () = server_stopping(&mut stopping) => return close_with(&mut socket, CLOSE_GOING_AWAY).await,
            incoming = socket.recv() => {
                let Some(Ok(Message::Text(text))) = incoming else {
                    return;
                };
                if !state.agent_authentication.is_live(&grant) {
                    return close_with(&mut socket, CLOSE_UNAUTHORIZED).await;
                }
                let answer = answer_request(&state, &grant, text.as_str()).await;
                if send_text(&mut socket, &answer.encode()).await.is_err() {
                    return;
                }
            }
        }
    }
}

async fn answer_request(state: &ServerState, grant: &AgentGrant, frame: &str) -> Response {
    let Some(request) = Request::decode(frame) else {
        return Response::failure(
            0,
            ErrorCode::Invalid,
            "the frame is not a request with an id and a type",
        );
    };
    let refusal = if ALLOWED_REQUESTS.contains(&request.kind.as_str()) {
        if let Some(field) = smuggled_field(&request.fields) {
            Refusal::SmuggledField(field)
        } else {
            match request.kind.as_str() {
                "agent.whoami" => {
                    return Response::success(
                        request.id,
                        json!({"agent_id": grant.agent_id.to_string()}),
                    );
                }
                "agent.subtree" => return answer_subtree(state, grant, &request).await,
                "directive.send" => return answer_directive(state, grant, &request).await,
                "report.send" => return answer_report(state, grant, &request).await,
                "agent.create" | "agent.start" | "agent.new_task" | "agent.stop" => {
                    return answer_office_request(state, grant, &request).await;
                }
                _ => Refusal::NotYet,
            }
        }
    } else {
        // `{:?}` quotes and escapes, so an embedded newline cannot fake a log
        // line of its own.
        state.log.refusal(&format!(
            "agent {} asked for {:?}, which an agent may not",
            grant.agent_id,
            truncated(&request.kind, MAX_REFUSAL_TEXT_CHARS)
        ));
        Refusal::NotAllowed
    };
    // Every refusal reaches the action record too (FR-008, FR-009), with the
    // caller as the actor.
    let reason = refusal_reason(&refusal, &request.kind);
    journal_refusal(state, grant.agent_id, &reason);
    Response::failure(request.id, ErrorCode::Refused, reason)
}

/// Journals a refusal and publishes the entry it wrote.
fn journal_refusal(state: &ServerState, caller: AgentId, reason: &str) {
    recorded_and_published(&state.events, || {
        state.directive_journal.refused(caller, reason);
    });
}

/// Runs `work`, then publishes the `action.recorded` events it caused to
/// every client. The recorder keeps them per thread, so the publish must run
/// on the thread that recorded; the entries an earlier request on this
/// thread left behind are dropped first, and only the entries of `work` go
/// out. A request forwarded to the `AgentGateway` publishes its own, so it
/// needs no wrap.
pub fn recorded_and_published<T>(events: &EventHub, work: impl FnOnce() -> T) -> T {
    drop(take_recorded_events());
    let outcome = work();
    for event in take_recorded_events() {
        events.publish(&event);
    }
    outcome
}

/// The name of the first field the frame carries that the server fixes
/// itself, on any allowed type: the actor comes from the credential, never
/// from the client.
fn smuggled_field(fields: &serde_json::Map<String, Value>) -> Option<String> {
    FORBIDDEN_FIELDS
        .iter()
        .find(|field| fields.contains_key(**field))
        .map(|field| (*field).to_owned())
}

/// The refusal's text: what the record keeps and the caller sees. A client
/// text in the reason (the type, a field key) is cut at the same 64
/// characters as the log line, so neither the log nor the record floods.
fn refusal_reason(refusal: &Refusal, kind: &str) -> String {
    match refusal {
        Refusal::NotAllowed => NOT_ALLOWED.to_owned(),
        Refusal::NotYet => format!(
            "{} is not available to agents yet",
            truncated(kind, MAX_REFUSAL_TEXT_CHARS)
        ),
        Refusal::SmuggledField(field) => format!(
            "the request carries the field {:?}, which the server does not take",
            truncated(field, MAX_REFUSAL_TEXT_CHARS)
        ),
    }
}

/// Answers `agent.subtree` (0014): the state of the caller's descendants.
/// The read takes no field, so it can point nowhere but at the caller's own
/// subtree (FR-002); a field is `invalid` and recorded (FR-005). It passes no
/// hold and no rate limit (FR-004), and an answered read writes no record
/// entry and publishes no event (FR-005).
async fn answer_subtree(state: &ServerState, grant: &AgentGrant, request: &Request) -> Response {
    let caller = grant.agent_id;
    if let Some(key) = request.fields.keys().next() {
        let message = format!(
            "agent.subtree does not take the field {:?}",
            truncated(key, MAX_REFUSAL_TEXT_CHARS)
        );
        journal_refusal(state, caller, &message);
        return Response::failure(request.id, ErrorCode::Invalid, message);
    }
    let gateway = state.agents.clone();
    let holds = Arc::clone(&state.hold_service);
    let read = spawn_blocking(move || {
        let view = holds.list();
        let holds: Vec<Hold> = view.office.into_iter().chain(view.agents).collect();
        gateway.subtree_status(caller, &holds)
    })
    .await;
    let message = match read {
        Ok(Ok(entries)) => return Response::success(request.id, entries),
        Ok(Err(message)) => truncated(&message, MAX_REFUSAL_TEXT_CHARS),
        Err(error) => format!("the request task failed: {error}"),
    };
    journal_refusal(state, caller, &message);
    Response::failure(request.id, ErrorCode::Internal, message)
}

/// Runs `directive.send` from the caller's actor: the parse failures answer
/// `invalid` and journal the refusal; the use case's own refusals answer as
/// the hierarchy mapped them and it journals the attempt itself.
async fn answer_directive(state: &ServerState, grant: &AgentGrant, request: &Request) -> Response {
    let (target, text) = match parse_directive(&request.fields) {
        Ok(parsed) => parsed,
        Err(message) => {
            journal_refusal(state, grant.agent_id, &message);
            return Response::failure(request.id, ErrorCode::Invalid, message);
        }
    };
    let sender = grant.agent_id;
    let handler = Arc::clone(&state.send_directive);
    let events = state.events.clone();
    let sent = spawn_blocking(move || {
        // Everything downstream, the journal included, reads the actor from
        // this scope, so the record names the calling agent (FR-011).
        let _actor = ActorScope::enter(Actor::Agent(sender));
        recorded_and_published(&events, || {
            handler
                .handle(sender, target, &text)
                .map_err(|error| (error.to_string(), wire_code(&error)))
        })
    })
    .await;
    match sent {
        Ok(outcome) => match outcome {
            Ok(delivery) => Response::success(request.id, delivery_json(delivery)),
            Err((message, code)) => Response::failure(request.id, code, message),
        },
        // The blocking task died before the use case could journal; the
        // gateway journals what it knows.
        Err(error) => {
            let message = format!("the request task failed: {error}");
            journal_refusal(state, sender, &message);
            Response::failure(request.id, ErrorCode::Internal, message)
        }
    }
}

/// The answer of an accepted directive (0012/FR-004): `submitted`, `waiting`
/// with the place in the target's queue counted from 1, or `unsubmitted`.
fn delivery_json(outcome: DirectiveOutcome) -> Value {
    match outcome {
        DirectiveOutcome::Submitted => json!({"delivery": "submitted"}),
        DirectiveOutcome::Waiting { position } => {
            json!({"delivery": "waiting", "position": position})
        }
        DirectiveOutcome::Unsubmitted => json!({"delivery": "unsubmitted"}),
    }
}

/// Runs `report.send` from the caller's actor (0015): the parse failures
/// answer `invalid` and journal the refusal; the use case journals its own
/// refusals and every accepted report.
async fn answer_report(state: &ServerState, grant: &AgentGrant, request: &Request) -> Response {
    let sender = grant.agent_id;
    let report = match parse_report(sender, &request.fields) {
        Ok(report) => report,
        Err(message) => {
            journal_refusal(state, sender, &message);
            return Response::failure(request.id, ErrorCode::Invalid, message);
        }
    };
    let handler = Arc::clone(&state.send_report);
    let events = state.events.clone();
    let sent = spawn_blocking(move || {
        let _actor = ActorScope::enter(Actor::Agent(sender));
        recorded_and_published(&events, || {
            handler
                .handle(&report)
                .map_err(|error| (error.to_string(), wire_code(&error)))
        })
    })
    .await;
    match sent {
        Ok(Ok(delivery)) => Response::success(request.id, report_answer(delivery)),
        Ok(Err((message, code))) => Response::failure(request.id, code, message),
        Err(error) => {
            let message = format!("the request task failed: {error}");
            journal_refusal(state, sender, &message);
            Response::failure(request.id, ErrorCode::Internal, message)
        }
    }
}

/// The answer of an accepted report (0015 contract, `report.send` result).
fn report_answer(delivery: ReportDelivery) -> Value {
    match delivery {
        ReportDelivery::NeedsYou => json!({"receiver": "person", "delivery": "needs_you"}),
    }
}

/// Reads `{kind, text, for_person?}` and nothing else: a field the type does
/// not take, such as one that names an agent, or a field of the wrong shape,
/// is `invalid` (0015/FR-002, the closed allowlist of the contract).
fn parse_report(
    sender: AgentId,
    fields: &serde_json::Map<String, Value>,
) -> Result<Report, String> {
    for key in fields.keys() {
        if !REPORT_FIELDS.contains(&key.as_str()) {
            return Err(format!(
                "report.send does not take the field {:?}",
                truncated(key, MAX_REFUSAL_TEXT_CHARS)
            ));
        }
    }
    let kind = fields
        .get("kind")
        .and_then(Value::as_str)
        .ok_or("kind is not one of done, question, blocked, info")?;
    let kind = ReportKind::from_wire(kind).map_err(|error| error.to_string())?;
    let text = fields
        .get("text")
        .and_then(Value::as_str)
        .ok_or("text is not a text")?
        .to_owned();
    let for_person = match fields.get("for_person") {
        None => false,
        Some(Value::Bool(for_person)) => *for_person,
        Some(_) => return Err("for_person is not true or false".to_owned()),
    };
    Ok(Report {
        sender,
        kind,
        text,
        for_person,
    })
}

/// Reads `{target_agent_id, text}` and nothing else: a field the type does
/// not take, or a field of the wrong shape, is `invalid` (the closed
/// allowlist of the 0002 contract).
fn parse_directive(fields: &serde_json::Map<String, Value>) -> Result<(AgentId, String), String> {
    for key in fields.keys() {
        if !DIRECTIVE_FIELDS.contains(&key.as_str()) {
            return Err(format!(
                "directive.send does not take the field {:?}",
                truncated(key, MAX_REFUSAL_TEXT_CHARS)
            ));
        }
    }
    let target = fields
        .get("target_agent_id")
        .and_then(Value::as_str)
        .and_then(AgentId::from_uuid_text)
        .ok_or("target_agent_id is not an agent id")?;
    let text = fields
        .get("text")
        .and_then(Value::as_str)
        .ok_or("text is not a text")?
        .to_owned();
    Ok((target, text))
}

/// Runs an office request (`agent.create`, `agent.start`, `agent.new_task`,
/// `agent.stop`) from the caller's actor (0002, FR-008). Before anything
/// runs, the request's own target must sit in the caller's subtree, checked
/// on a fresh tree snapshot; the answer is the `AgentGateway`'s own, so the
/// request behaves like the user had sent it (PB-1), under the caller's
/// actor. `agent.delete` never reaches here: it is outside the allowlist.
async fn answer_office_request(
    state: &ServerState,
    grant: &AgentGrant,
    request: &Request,
) -> Response {
    let caller = grant.agent_id;
    // The field allowlist runs before anything else: a frame the type does
    // not take is refused as `invalid` before any rule reads its fields
    // (the same shape as `parse_directive`, PR 40 LOW).
    if let Err(message) = refuse_extra_fields(&request.kind, &request.fields) {
        let message = truncated(&message, MAX_REFUSAL_TEXT_CHARS);
        journal_refusal(state, caller, &message);
        return Response::failure(request.id, ErrorCode::Invalid, message);
    }
    let target = match target_of(request, caller) {
        Ok(target) => target,
        Err(message) => {
            let message = truncated(&message, MAX_REFUSAL_TEXT_CHARS);
            journal_refusal(state, caller, &message);
            return Response::failure(request.id, ErrorCode::Invalid, message);
        }
    };
    // An agent does not start, task, or stop itself over agent-call: the
    // contract keeps start, new_task, and stop to descendants only (FR-008).
    // Only `agent.create` may name the caller — as the default parent of the
    // child — so the self case cannot reach the tree check below.
    if request.kind != "agent.create" && target == caller {
        let reason = "an agent does not act on itself".to_owned();
        state.log.refusal(&format!(
            "agent {caller} asked for {:?} on itself",
            truncated(&request.kind, MAX_REFUSAL_TEXT_CHARS)
        ));
        journal_refusal(state, caller, &reason);
        return Response::failure(request.id, ErrorCode::Refused, reason);
    }
    // The gates run on a fresh snapshot, on the blocking pool, before the
    // gateway sees the request: an agent that names a stranger (as a target
    // or as the parent of its create) reaches no use case at all (FR-008,
    // FR-009), and a create's role must be one the caller's own place allows
    // (A4). The caller itself is always inside its own subtree, placed or
    // not — a member may create before it is placed, and the role rules then
    // refuse what its place cannot pass. Past the self refusal above, a
    // target equal to the caller is a create parent.
    let tree_query = Arc::clone(&state.hierarchy_tree);
    let kind = request.kind.clone();
    let fields_in = request.fields.clone();
    let inside = spawn_blocking(move || {
        tree_query.store_snapshot().map(|rows| {
            let target_ok = target == caller || check_descendant(&rows, caller, target).is_ok();
            let fields = allowed_fields_of(&kind, &fields_in, caller, &rows);
            (target_ok, fields, rows)
        })
    })
    .await;
    let (descendant_ok, fields, rows) = match inside {
        Ok(Ok((target_ok, fields, rows))) => (
            target_ok,
            fields.map_err(|(code, message)| (code, truncated(&message, MAX_REFUSAL_TEXT_CHARS))),
            rows,
        ),
        Ok(Err(error)) => {
            let message = truncated(&error.to_string(), MAX_REFUSAL_TEXT_CHARS);
            journal_refusal(state, caller, &message);
            return Response::failure(request.id, wire_code(&error), message);
        }
        Err(error) => {
            let message = format!("the request task failed: {error}");
            journal_refusal(state, caller, &message);
            return Response::failure(request.id, ErrorCode::Internal, message);
        }
    };
    if !descendant_ok {
        return refuse_outside_subtree(state, caller, request);
    }
    let fields = match fields {
        Ok(fields) => fields,
        Err((code, message)) => {
            journal_refusal(state, caller, &message);
            return Response::failure(request.id, code, message);
        }
    };
    // The hold and the rate limits come last, after every 0002 rule, on the
    // same snapshot the rules read (spec A10).
    if let Err(response) = guard_office_request(state, caller, request, target, rows).await {
        return response;
    }
    let forwarded = Request {
        id: request.id,
        kind: request.kind.clone(),
        fields,
    };
    let gateway = state.agents.clone();
    let actor = caller;
    let answered = spawn_blocking(move || {
        // The gateway's handlers journal and record under the actor this
        // scope names, so the record keeps the calling agent (FR-011).
        let _scope = ActorScope::enter(Actor::Agent(actor));
        gateway.run(&forwarded)
    })
    .await;
    match answered {
        Ok(response) => {
            // A refused create is a refusal like a refused directive, so it
            // reaches the record with the caller as the actor (FR-009). The
            // other office types record their own failures (a Stop or
            // NewTask entry carries the failure text); the create use case
            // keeps the record for its successes only, so the gateway
            // writes the refused ones.
            if request.kind == "agent.create"
                && let Response::Failure { message, .. } = &response
            {
                journal_refusal(state, caller, message);
            }
            response
        }
        Err(error) => {
            let message = format!("the request task failed: {error}");
            journal_refusal(state, caller, &message);
            Response::failure(request.id, ErrorCode::Internal, message)
        }
    }
}

/// Answers, logs, and journals a request whose target lies outside the
/// caller's subtree (FR-008).
fn refuse_outside_subtree(state: &ServerState, caller: AgentId, request: &Request) -> Response {
    let reason = format!(
        "the agent is not a descendant of {}",
        truncated(&caller.to_string(), MAX_REFUSAL_TEXT_CHARS)
    );
    state.log.refusal(&format!(
        "agent {caller} asked for {:?} outside its subtree",
        truncated(&request.kind, MAX_REFUSAL_TEXT_CHARS)
    ));
    journal_refusal(state, caller, &reason);
    Response::failure(request.id, ErrorCode::Refused, reason)
}

/// Asks the hold service whether an office request may run: a create counts
/// toward the create limit with its parent as the hold target, the other
/// kinds toward the lifecycle limit with the named agent as the target. The
/// refusal is journaled with the caller as actor, the kind, and the target
/// (FR-010); the answer carries the reason alone.
async fn guard_office_request(
    state: &ServerState,
    caller: AgentId,
    request: &Request,
    target: AgentId,
    rows: Vec<TreeRow>,
) -> Result<(), Response> {
    let action = if request.kind == "agent.create" {
        AgentAction::Create
    } else {
        AgentAction::Lifecycle
    };
    let holds = Arc::clone(&state.hold_service);
    let guarded = spawn_blocking(move || holds.guard(caller, action, Some(target), &rows)).await;
    let refusal = match guarded {
        Ok(Ok(())) => return Ok(()),
        Ok(Err(refusal)) => refusal,
        Err(error) => {
            let message = format!("the request task failed: {error}");
            journal_refusal(state, caller, &message);
            return Err(Response::failure(request.id, ErrorCode::Internal, message));
        }
    };
    journal_refusal(
        state,
        caller,
        &format!(
            "{} on {target}: {}",
            truncated(&request.kind, MAX_REFUSAL_TEXT_CHARS),
            refusal.reason
        ),
    );
    Err(Response::failure(
        request.id,
        ErrorCode::Refused,
        refusal.reason,
    ))
}

/// The agent the request acts on: the named `agent_id`, or, for a create,
/// the parent the new agent would sit under — the requested one, else the
/// caller. That parent must sit in the caller's subtree, the same rule the
/// other types apply to their target (FR-008); a create with no parent and
/// no parent-naming role passes the caller itself, so an unplaced lead's
/// create still reaches the role rules with the caller's own place.
fn target_of(request: &Request, caller: AgentId) -> Result<AgentId, String> {
    if request.kind == "agent.create" {
        let requested = request
            .fields
            .get("role")
            .and_then(|role| role.get("parent_agent_id"))
            .and_then(Value::as_str);
        return Ok(match requested {
            Some(text) => {
                AgentId::from_uuid_text(text).ok_or("role.parent_agent_id is not an agent id")?
            }
            None => caller,
        });
    }
    let text = request
        .fields
        .get("agent_id")
        .and_then(Value::as_str)
        .ok_or("agent_id is required")?;
    AgentId::from_uuid_text(text).ok_or_else(|| "agent_id is not an agent id".to_owned())
}

/// The fields each office type takes, and no others: a field the type does
/// not take is refused, not dropped — the same closed allowlist as
/// `directive.send` (PR 40 LOW: a silently trimmed `include_descendants`
/// answered success while stopping the target alone). A create also takes
/// the `role` object, which `allowed_fields_of` validates and replaces, so
/// the allowlist is `CREATE_FIELDS` plus that one key.
///
/// # Errors
/// The name of the first field the type does not take.
fn refuse_extra_fields(kind: &str, fields: &serde_json::Map<String, Value>) -> Result<(), String> {
    for key in fields.keys() {
        let taken = if kind == "agent.create" {
            CREATE_FIELDS.contains(&key.as_str()) || key == "role"
        } else {
            key == "agent_id"
        };
        if !taken {
            return Err(format!(
                "{kind} does not take the field {:?}",
                truncated(key, MAX_REFUSAL_TEXT_CHARS)
            ));
        }
    }
    Ok(())
}

/// The fields an office request keeps for the gateway: a create keeps the
/// fields the agent protocol takes, with the client's own `role` after the
/// caller's place has allowed it against the tree rows (A4, FR-009); the
/// other types keep only `agent_id`. The frame's keys are already the type's
/// own (`refuse_extra_fields` ran first), so nothing is dropped here.
///
/// # Errors
/// The create refused its role: the code is `invalid` for a role the wire
/// cannot read, `refused` for one the caller's place does not allow.
fn allowed_fields_of(
    kind: &str,
    fields_in: &serde_json::Map<String, Value>,
    caller: AgentId,
    rows: &[TreeRow],
) -> Result<serde_json::Map<String, Value>, (ErrorCode, String)> {
    let mut fields = serde_json::Map::new();
    if kind == "agent.create" {
        for key in CREATE_FIELDS {
            if let Some(value) = fields_in.get(key) {
                fields.insert((*key).to_owned(), value.clone());
            }
        }
        // The client's role crosses, but only a role the caller's place
        // allows (A4): a CEO raises a lead, whose team the caller names,
        // a lead raises members under itself or under its own descendant
        // leads, a member and an unplaced agent raise nothing, and no agent
        // creates a CEO. A create without a role keeps the plain create's
        // meaning on this connection: a member under the caller itself.
        let (role_kind, parent, team) = match fields_in.get("role") {
            None | Some(Value::Null) => (Role::Member.as_str(), Some(caller), None),
            Some(role) => role_of(role).map_err(|message| (ErrorCode::Invalid, message))?,
        };
        let (parent, team) = validated_place(role_kind, parent, team, caller, rows)?;
        let mut role = serde_json::Map::new();
        role.insert("kind".to_owned(), Value::String(role_kind.to_owned()));
        if let Some(parent) = parent {
            role.insert(
                "parent_agent_id".to_owned(),
                Value::String(parent.to_string()),
            );
        }
        if let Some(team) = team {
            role.insert("team_id".to_owned(), Value::String(team.as_uuid_text()));
        }
        fields.insert("role".to_owned(), Value::Object(role));
    } else if let Some(agent_id) = fields_in.get("agent_id") {
        fields.insert("agent_id".to_owned(), agent_id.clone());
    }
    Ok(fields)
}

/// The fields the `role` object takes, and no others: a key outside this
/// list is `invalid` before any place rule reads the role (the same closed
/// allowlist the frame itself passes, PR 40 r2 LOW).
const ROLE_FIELDS: [&str; 3] = ["kind", "parent_agent_id", "team_id"];

/// Reads the client's `role` object as the triple the place rules decide on:
/// the kind in its wire form, the parent it names, and the team it names, if
/// any. A role that is not an object, a kind that is not one of the three
/// roles, or a field the role does not take, names the field at fault.
fn role_of(role: &Value) -> Result<(&str, Option<AgentId>, Option<TeamId>), String> {
    let Value::Object(role) = role else {
        return Err("role must be an object".to_owned());
    };
    for key in role.keys() {
        if !ROLE_FIELDS.contains(&key.as_str()) {
            return Err(format!(
                "role does not take the field {:?}",
                truncated(key, MAX_REFUSAL_TEXT_CHARS)
            ));
        }
    }
    let kind = role
        .get("kind")
        .and_then(Value::as_str)
        .ok_or("role.kind is required")?;
    let kind = Role::from_wire(kind)
        .map(Role::as_str)
        .ok_or_else(|| format!("role.kind is not a role of the company: {kind}"))?;
    let parent = role
        .get("parent_agent_id")
        .and_then(Value::as_str)
        .map(|text| AgentId::from_uuid_text(text).ok_or("role.parent_agent_id is not an agent id"))
        .transpose()?;
    let team = role
        .get("team_id")
        .and_then(Value::as_str)
        .map(|text| TeamId::from_uuid_text(text).ok_or("role.team_id is not a team id"))
        .transpose()?;
    Ok((kind, parent, team))
}

/// The child roles an agent caller may ask for, from its own place in the
/// tree (A4): a CEO raises a lead, whose team the caller names (the team is
/// the lead's place; the parent comes from the tree), and a lead raises a
/// member under itself. A member may also land under one of the caller's
/// descendant leads, so a CEO can fill a team it can see; the tree rules
/// still check the place the child takes. A member or an unplaced agent
/// raises nothing, and no agent creates a CEO.
///
/// # Errors
/// `refused` with the roles the caller's place allows, so the agent knows
/// what it may ask for instead (FR-009); `invalid` for a place a lead child
/// cannot take.
fn validated_place(
    kind: &str,
    parent: Option<AgentId>,
    team: Option<TeamId>,
    caller: AgentId,
    rows: &[TreeRow],
) -> Result<(Option<AgentId>, Option<TeamId>), (ErrorCode, String)> {
    if kind == Role::Ceo.as_str() {
        return Err((ErrorCode::Refused, CEO_REFUSAL.to_owned()));
    }
    let caller_role = rows
        .iter()
        .find(|row| row.agent_id == caller)
        .and_then(|row| row.role);
    let Some(allowed) = caller_role.map(Role::allowed_child_roles) else {
        return Err((ErrorCode::Refused, UNPLACED_REFUSAL.to_owned()));
    };
    if kind == Role::Lead.as_str() {
        child_role_allowed(kind, allowed)?;
        if parent.is_some() {
            return Err((
                ErrorCode::Invalid,
                "role.parent_agent_id must stay unset for a lead".to_owned(),
            ));
        }
        // The team is the lead's place (0002 contract: a lead REQUIRES a
        // team_id). Without one the lead would be placed outside every
        // team, hidden from the tree, so the gateway refuses instead of
        // answering success for a lead the office cannot show.
        let team = team.ok_or((
            ErrorCode::Invalid,
            "role.team_id is required for a lead; the team is the lead's place".to_owned(),
        ))?;
        return Ok((None, Some(team)));
    }
    // A member's team is its lead's team (A6), so the caller names no team
    // for it; a named team would read as a place the lead does not give.
    if team.is_some() {
        return Err((
            ErrorCode::Invalid,
            "role.team_id must stay unset for a member; the member takes its lead's team"
                .to_owned(),
        ));
    }
    // The member lands under the caller, or under a lead of the caller's
    // own subtree when one is named (FR-008): under anything else the tree
    // rules would refuse, so the gateway refuses first with its reason.
    let parent = parent.unwrap_or(caller);
    if parent == caller {
        child_role_allowed(kind, allowed)?;
        return Ok((Some(parent), None));
    }
    let parent_is_lead = rows
        .iter()
        .any(|row| row.agent_id == parent && row.role == Some(Role::Lead));
    if !is_descendant_of(rows, parent, caller) {
        return Err((ErrorCode::Refused, PARENT_OUTSIDE_REFUSAL.to_owned()));
    }
    if !parent_is_lead {
        return Err((ErrorCode::Refused, PARENT_NOT_LEAD_REFUSAL.to_owned()));
    }
    Ok((Some(parent), None))
}

/// Why an agent's create without a role of its own is refused: an unplaced
/// agent (a role of `None`) can pass no place to a child (A4).
const UNPLACED_REFUSAL: &str =
    "an agent without a place in the company may not create agents; ask the user for a place first";

/// Why a CEO role is never an agent's to give: the office has one CEO, and
/// only the user creates it (A4, A5).
const CEO_REFUSAL: &str = "no agent may create a ceo; only the user creates the company's ceo";

/// Why a member's create is refused whatever it asks for: a member has no
/// child agents in the office (A4, A3).
const MEMBER_REFUSAL: &str = "a member may not create agents; its place allows no child roles";

/// Why a member parent outside the caller's subtree is refused: an agent
/// raises children only within its own descendants (FR-008).
const PARENT_OUTSIDE_REFUSAL: &str =
    "the parent of the new agent is not a descendant of the caller";

/// Why a named parent that is not a lead is refused: a member sits only
/// under a lead (A6), and under the caller the role rules say it anyway.
const PARENT_NOT_LEAD_REFUSAL: &str =
    "the parent of the new member is not a lead; a member sits under a lead";

/// Whether the caller's place allows the child role at all: a place with no
/// child roles (a member) refuses with its own reason, one with child roles
/// that do not include the kind refuses with the list (A4, FR-009).
///
/// # Errors
/// Always `refused`, with the reason the caller's place gives.
fn child_role_allowed(kind: &str, allowed: &[Role]) -> Result<(), (ErrorCode, String)> {
    if allowed.iter().any(|role| role.as_str() == kind) {
        return Ok(());
    }
    Err((
        ErrorCode::Refused,
        if allowed.is_empty() {
            MEMBER_REFUSAL.to_owned()
        } else {
            allowed_roles_refusal(allowed)
        },
    ))
}

/// The roles an agent's place allows, as the refusal names them (the same
/// form the tree rules refuse with, FR-003).
fn allowed_roles_refusal(allowed: &[Role]) -> String {
    let allowed = allowed
        .iter()
        .map(|role| role.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!("the caller's place allows only these child roles: {allowed}")
}

/// The fields of `agent.create` an agent caller may set. `role` joins the
/// frame beside them and is validated against the caller's place (A4);
/// `additions` is refused outright, named by the gate like any other field
/// the type does not take.
const CREATE_FIELDS: [&str; 4] = ["name", "command", "working_folder", "project_id"];

/// The wire code of a directive failure: a rule between the agents is a
/// refusal (the contract names not-a-descendant and not-running `refused`),
/// a rule about the text is `invalid`, and the writer's own failure is the
/// conflict it carries.
fn wire_code(error: &HierarchyApplicationError) -> ErrorCode {
    match error {
        // A hold or a rate limit refused the action (0007).
        HierarchyApplicationError::Invalid(
            HierarchyError::NotADescendant { .. } | HierarchyError::NotRunning { .. },
        )
        | HierarchyApplicationError::Refused(_) => ErrorCode::Refused,
        HierarchyApplicationError::Invalid(_) | HierarchyApplicationError::InvalidInput(_) => {
            ErrorCode::Invalid
        }
        // The `*Input` variants carry an agent-module message, so they keep
        // the code of their kind — only the prefix is bare.
        HierarchyApplicationError::NotFoundInput(_) | HierarchyApplicationError::NotFound(_) => {
            ErrorCode::NotFound
        }
        HierarchyApplicationError::ConflictInput(_) | HierarchyApplicationError::Conflict(_) => {
            ErrorCode::Conflict
        }
        HierarchyApplicationError::Storage(_) => ErrorCode::Internal,
    }
}

/// Cuts `text` at `max` characters and adds one ellipsis when cut. The count
/// is characters, so a cut never splits a character in half.
fn truncated(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let mut cut: String = text.chars().take(max).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
#[path = "agent_call.tests.rs"]
mod tests;

async fn server_stopping(shutdown: &mut watch::Receiver<bool>) {
    // A dropped sender also means the server is going away.
    let _ = shutdown.wait_for(|stopping| *stopping).await;
}
