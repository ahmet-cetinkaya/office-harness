// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use access_application::commands::authenticate_agent_call::AgentGrant;
use agent_application::common::actor_scope::ActorScope;
use axum::extract::ws::{Message, WebSocket};
use hierarchy_application::common::errors::HierarchyApplicationError;
use hierarchy_domain::{HierarchyError, Role, TreeRow, check_descendant, is_descendant_of};
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
    let target = match target_of(request, caller) {
        Ok(target) => target,
        Err(message) => {
            let message = truncated(&message, MAX_REFUSAL_TEXT_CHARS);
            state.directive_journal.refused(caller, &message);
            return Response::failure(request.id, ErrorCode::Invalid, message);
        }
    };
    // The gates run on a fresh snapshot, on the blocking pool, before the
    // gateway sees the request: an agent that names a stranger (as a target
    // or as the parent of its create) reaches no use case at all (FR-008,
    // FR-009), and a create's role must be one the caller's own place allows
    // (A4). The caller itself is always inside its own subtree, placed or
    // not — a member may create before it is placed, and the role rules then
    // refuse what its place cannot pass.
    let tree_query = Arc::clone(&state.hierarchy_tree);
    let kind = request.kind.clone();
    let fields_in = request.fields.clone();
    let inside = spawn_blocking(move || {
        tree_query.store_snapshot().map(|rows| {
            let target_ok = target == caller || check_descendant(&rows, caller, target).is_ok();
            let role = allowed_fields_of(&kind, &fields_in, caller, &rows);
            (target_ok, role)
        })
    })
    .await;
    let (descendant_ok, fields) = match inside {
        Ok(Ok((target_ok, role))) => (
            target_ok,
            role.map_err(|(code, message)| (code, truncated(&message, MAX_REFUSAL_TEXT_CHARS))),
        ),
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
    let fields = match fields {
        Ok(fields) => fields,
        Err((code, message)) => {
            state.directive_journal.refused(caller, &message);
            return Response::failure(request.id, code, message);
        }
    };
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

/// The fields an office request may carry: a create keeps the fields the
/// agent protocol takes, with the client's own `role` after the caller's
/// place has allowed it against the tree rows (A4, FR-009); the other types
/// keep only `agent_id`. Any other field is dropped before the gateway sees
/// the frame (the closed allowlist of the 0002 contract).
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
        // An agent never sets the additions of its child: the office
        // additions belong to the user's own judgement (0002 contract, agent
        // caller).
        if fields_in.contains_key("additions") {
            return Err((
                ErrorCode::Invalid,
                "agent.create does not take the field \"additions\"".to_owned(),
            ));
        }
        for key in CREATE_FIELDS {
            if let Some(value) = fields_in.get(key) {
                fields.insert((*key).to_owned(), value.clone());
            }
        }
        // The client's role crosses, but only a role the caller's place
        // allows (A4): a CEO raises leads, a lead raises members under
        // itself or under its own descendant leads, a member and an unplaced
        // agent raise nothing, and no agent creates a CEO. A create without
        // a role keeps the plain create's meaning on this connection: a
        // member under the caller itself.
        let (role_kind, parent) = match fields_in.get("role") {
            None | Some(Value::Null) => (Role::Member.as_str(), Some(caller)),
            Some(role) => role_of(role).map_err(|message| (ErrorCode::Invalid, message))?,
        };
        let parent = validated_parent(role_kind, parent, caller, rows)?;
        let mut role = serde_json::Map::new();
        role.insert("kind".to_owned(), Value::String(role_kind.to_owned()));
        if let Some(parent) = parent {
            role.insert(
                "parent_agent_id".to_owned(),
                Value::String(parent.to_string()),
            );
        }
        fields.insert("role".to_owned(), Value::Object(role));
    } else if let Some(agent_id) = fields_in.get("agent_id") {
        fields.insert("agent_id".to_owned(), agent_id.clone());
    }
    Ok(fields)
}

/// Reads the client's `role` object as the pair the place rules decide on:
/// the kind in its wire form and the parent it names, if any. A role that is
/// not an object, or whose kind is not one of the three roles, names the
/// field at fault.
fn role_of(role: &Value) -> Result<(&str, Option<AgentId>), String> {
    let Value::Object(role) = role else {
        return Err("role must be an object".to_owned());
    };
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
    Ok((kind, parent))
}

/// The child roles an agent caller may ask for, from its own place in the
/// tree (A4): a CEO raises a lead (the lead's place comes from the CEO and
/// its team from the user, so the caller names no parent for it), and a lead
/// raises a member under itself. A member may also land under one of the
/// caller's descendant leads, so a CEO can fill a team it can see; the tree
/// rules still check the place the child takes. A member or an unplaced
/// agent raises nothing, and no agent creates a CEO.
///
/// # Errors
/// `refused` with the roles the caller's place allows, so the agent knows
/// what it may ask for instead (FR-009); `invalid` for a parent a lead
/// child cannot take.
fn validated_parent(
    kind: &str,
    parent: Option<AgentId>,
    caller: AgentId,
    rows: &[TreeRow],
) -> Result<Option<AgentId>, (ErrorCode, String)> {
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
        return Ok(None);
    }
    // The member lands under the caller, or under a lead of the caller's
    // own subtree when one is named (FR-008): under anything else the tree
    // rules would refuse, so the gateway refuses first with its reason.
    let parent = parent.unwrap_or(caller);
    if parent == caller {
        child_role_allowed(kind, allowed)?;
        return Ok(Some(parent));
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
    Ok(Some(parent))
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
/// frame separately, validated against the caller's place (A4), and
/// `additions` is refused outright.
const CREATE_FIELDS: [&str; 4] = ["name", "command", "working_folder", "project_id"];

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
        HierarchyApplicationError::NotFound(_) => ErrorCode::NotFound,
        HierarchyApplicationError::Conflict(_) => ErrorCode::Conflict,
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
