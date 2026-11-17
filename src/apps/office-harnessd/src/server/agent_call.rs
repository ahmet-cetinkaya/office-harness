// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use access_application::commands::authenticate_agent_call::AgentGrant;
use agent_application::common::actor_scope::ActorScope;
use axum::extract::ws::{Message, WebSocket};
use hierarchy_application::common::errors::HierarchyApplicationError;
use hierarchy_domain::HierarchyError;
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

/// The longest request type the refusal log keeps. The type is the agent's
/// text, so the limit keeps one hostile frame from flooding the log.
const MAX_LOGGED_KIND_CHARS: usize = 64;

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
                _ => Refusal::NotYet,
            }
        }
    } else {
        // `{:?}` quotes and escapes, so an embedded newline cannot fake a log
        // line of its own.
        state.log.refusal(&format!(
            "agent {} asked for {:?}, which an agent may not",
            grant.agent_id,
            truncated(&request.kind, MAX_LOGGED_KIND_CHARS)
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

/// The refusal's text: what the record keeps and the caller sees.
fn refusal_reason(refusal: &Refusal, kind: &str) -> String {
    match refusal {
        Refusal::NotAllowed => NOT_ALLOWED.to_owned(),
        Refusal::NotYet => format!("{kind} is not available to agents yet"),
        Refusal::SmuggledField(field) => {
            format!("the request carries the field {field:?}, which the server does not take")
        }
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
            return Err(format!("directive.send does not take the field {key:?}"));
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

/// The wire code of a directive failure: a rule between the agents is a
/// refusal (the contract names not-a-descendant and not-running `refused`),
/// a rule about the text is `invalid`, and the writer's own failure is the
/// conflict it carries.
fn wire_code(error: &HierarchyApplicationError) -> ErrorCode {
    match error {
        HierarchyApplicationError::Invalid(
            HierarchyError::NotADescendant { .. } | HierarchyError::NotRunning { .. },
        ) => ErrorCode::Refused,
        HierarchyApplicationError::Invalid(_) => ErrorCode::Invalid,
        HierarchyApplicationError::NotFound(_) => ErrorCode::NotFound,
        HierarchyApplicationError::Conflict(_) => ErrorCode::Conflict,
        HierarchyApplicationError::Storage => ErrorCode::Internal,
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
