// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use access_application::commands::authenticate_agent_call::AgentGrant;
use agent_application::common::actor_scope::ActorScope;
use axum::extract::ws::{Message, WebSocket};
use hierarchy_application::common::errors::HierarchyApplicationError;
use hierarchy_domain::{HierarchyError, check_descendant};
use kernel::{Actor, AgentId};
use serde_json::{Value, json};
use tokio::sync::watch;
use tokio::task::spawn_blocking;
use wire::{CLOSE_UNAUTHORIZED, ErrorCode, Request, Response};

use super::connection::{CLOSE_GOING_AWAY, close_with, send_text};
use super::server_state::ServerState;

/// The only request types an agent connection answers (0002, plan D3). Any
/// other type is refused, so no agent reaches `terminal.*`, `agent.delete`,
/// `team.*`, `hierarchy.*`, `server.*`, `settings.*`, `project.*`, or the
/// additions of an agent.
const ALLOWED_REQUESTS: [&str; 6] = [
    "agent.whoami",
    "agent.create",
    "agent.start",
    "agent.new_task",
    "agent.stop",
    "directive.send",
];

/// What the connection of an agent says about a request it will not run.
const NOT_ALLOWED: &str = "an agent may only create, start, stop, and give new tasks to its own descendants, and send them directives";

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
                "directive.send" => return answer_directive(state, grant, &request).await,
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
    state.directive_journal.refused(grant.agent_id, &reason);
    Response::failure(request.id, ErrorCode::Refused, reason)
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

/// Runs `directive.send` from the caller's actor: the parse failures answer
/// `invalid` and journal the refusal; the use case's own refusals answer as
/// the hierarchy mapped them and it journals the attempt itself.
async fn answer_directive(state: &ServerState, grant: &AgentGrant, request: &Request) -> Response {
    let (target, text) = match parse_directive(&request.fields) {
        Ok(parsed) => parsed,
        Err(message) => {
            state.directive_journal.refused(grant.agent_id, &message);
            return Response::failure(request.id, ErrorCode::Invalid, message);
        }
    };
    let sender = grant.agent_id;
    let handler = Arc::clone(&state.send_directive);
    let journal = Arc::clone(&state.directive_journal);
    let sent = spawn_blocking(move || {
        // Everything downstream, the journal included, reads the actor from
        // this scope, so the record names the calling agent (FR-011).
        let _actor = ActorScope::enter(Actor::Agent(sender));
        handler
            .handle(sender, target, &text)
            .map_err(|error| (error.to_string(), wire_code(&error)))
    })
    .await;
    match sent {
        Ok(outcome) => match outcome {
            Ok(()) => Response::success(request.id, Value::Null),
            Err((message, code)) => Response::failure(request.id, code, message),
        },
        // The blocking task died before the use case could journal; the
        // gateway journals what it knows.
        Err(error) => {
            let message = format!("the request task failed: {error}");
            journal.refused(sender, &message);
            Response::failure(request.id, ErrorCode::Internal, message)
        }
    }
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
    let fields = match allowed_fields_of(request, caller) {
        Ok(fields) => fields,
        Err(message) => {
            let message = truncated(&message, MAX_REFUSAL_TEXT_CHARS);
            state.directive_journal.refused(caller, &message);
            return Response::failure(request.id, ErrorCode::Invalid, message);
        }
    };
    let target = match target_of(request, caller) {
        Ok(target) => target,
        Err(message) => {
            let message = truncated(&message, MAX_REFUSAL_TEXT_CHARS);
            state.directive_journal.refused(caller, &message);
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
        state.directive_journal.refused(caller, &reason);
        return Response::failure(request.id, ErrorCode::Refused, reason);
    }
    // The descendant rule runs on a fresh snapshot, on the blocking pool,
    // before the gateway sees the request: an agent that names a stranger
    // (as a target or as the parent of its create) reaches no use case at
    // all (FR-008, FR-009). The caller itself is always inside its own
    // subtree, placed or not — a member may create before it is placed,
    // and the tree rules then decide the place the child takes. Past the
    // self refusal above, a target equal to the caller is a create parent.
    let tree_query = Arc::clone(&state.hierarchy_tree);
    let inside = spawn_blocking(move || {
        tree_query
            .store_snapshot()
            .map(|rows| target == caller || check_descendant(&rows, caller, target).is_ok())
    })
    .await;
    let descendant_ok = match inside {
        Ok(Ok(inside)) => inside,
        Ok(Err(error)) => {
            let message = truncated(&error.to_string(), MAX_REFUSAL_TEXT_CHARS);
            state.directive_journal.refused(caller, &message);
            return Response::failure(request.id, wire_code(&error), message);
        }
        Err(error) => {
            let message = format!("the request task failed: {error}");
            state.directive_journal.refused(caller, &message);
            return Response::failure(request.id, ErrorCode::Internal, message);
        }
    };
    if !descendant_ok {
        let reason = format!(
            "the agent is not a descendant of {}",
            truncated(&caller.to_string(), MAX_REFUSAL_TEXT_CHARS)
        );
        state.log.refusal(&format!(
            "agent {caller} asked for {:?} outside its subtree",
            truncated(&request.kind, MAX_REFUSAL_TEXT_CHARS)
        ));
        state.directive_journal.refused(caller, &reason);
        return Response::failure(request.id, ErrorCode::Refused, reason);
    }
    let forwarded = Request {
        id: request.id,
        kind: request.kind.clone(),
        fields,
    };
    let gateway = state.agents.clone();
    let journaled = Arc::clone(&state.directive_journal);
    let actor = caller;
    let answered = spawn_blocking(move || {
        // The gateway's handlers journal and record under the actor this
        // scope names, so the record keeps the calling agent (FR-011).
        let _scope = ActorScope::enter(Actor::Agent(actor));
        gateway.run(&forwarded)
    })
    .await;
    match answered {
        Ok(response) => response,
        Err(error) => {
            let message = format!("the request task failed: {error}");
            journaled.refused(caller, &message);
            Response::failure(request.id, ErrorCode::Internal, message)
        }
    }
}

/// The agent the request acts on: the named `agent_id`, or, for a create,
/// the parent the new agent would sit under — the requested one, else the
/// caller (`creation_parent`). That parent must sit in the caller's subtree,
/// the same rule the other types apply to their target (FR-008).
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

/// The fields an office request may carry: a create keeps the fields the
/// agent protocol takes, but the role is the server's, not the client's;
/// the other types keep only `agent_id`. Any other field is refused, not
/// dropped — the same closed allowlist as `directive.send` (Efe, PR 40 LOW:
/// a silently dropped `include_descendants` would answer success while
/// doing something else than asked).
fn allowed_fields_of(
    request: &Request,
    caller: AgentId,
) -> Result<serde_json::Map<String, Value>, String> {
    let mut fields = serde_json::Map::new();
    if request.kind == "agent.create" {
        // An agent never sets the additions of its child: the office
        // additions belong to the user's own judgement (0002 contract, agent
        // caller).
        if request.fields.contains_key("additions") {
            return Err("agent.create does not take the field \"additions\"".to_owned());
        }
        for key in request.fields.keys() {
            if !CREATE_ALLOWED.contains(&key.as_str()) {
                return Err(format!(
                    "agent.create does not take the field {:?}",
                    truncated(key, MAX_REFUSAL_TEXT_CHARS)
                ));
            }
        }
        for key in CREATE_FIELDS {
            if let Some(value) = request.fields.get(key) {
                fields.insert((*key).to_owned(), value.clone());
            }
        }
        // The child an agent creates is a member under the caller (A4,
        // FR-008): the parent stays the caller itself, the team is the
        // caller's place, and the client's own `role` object never crosses,
        // because an agent creates no CEO and no lead.
        let mut role = serde_json::Map::new();
        role.insert("kind".to_owned(), Value::String("member".to_owned()));
        role.insert(
            "parent_agent_id".to_owned(),
            Value::String(caller.to_string()),
        );
        fields.insert("role".to_owned(), Value::Object(role));
    } else {
        for key in request.fields.keys() {
            if key != "agent_id" {
                return Err(format!(
                    "{:?} does not take the field {:?}",
                    request.kind,
                    truncated(key, MAX_REFUSAL_TEXT_CHARS)
                ));
            }
        }
        if let Some(agent_id) = request.fields.get("agent_id") {
            fields.insert("agent_id".to_owned(), agent_id.clone());
        }
    }
    Ok(fields)
}

/// The fields of `agent.create` an agent caller may set. `role` is checked
/// against the allowlist here and joined separately; the client's own
/// `role` object is replaced, and `additions` is refused outright.
const CREATE_FIELDS: [&str; 4] = ["name", "command", "working_folder", "project_id"];

/// Everything a create frame may name: the seed fields plus the `role`
/// object that the allowlist check accepts and then replaces.
const CREATE_ALLOWED: [&str; 5] = ["name", "command", "working_folder", "project_id", "role"];

/// The wire code of a directive failure: a rule between the agents is a
/// refusal (the contract names not-a-descendant and not-running `refused`),
/// a rule about the text is `invalid`, and the writer's own failure is the
/// conflict it carries.
fn wire_code(error: &HierarchyApplicationError) -> ErrorCode {
    match error {
        HierarchyApplicationError::Invalid(
            HierarchyError::NotADescendant { .. } | HierarchyError::NotRunning { .. },
        ) => ErrorCode::Refused,
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
