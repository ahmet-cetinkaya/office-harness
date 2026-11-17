// SPDX-License-Identifier: AGPL-3.0-or-later

use access_application::commands::authenticate_agent_call::AgentGrant;
use axum::extract::ws::{Message, WebSocket};
use serde_json::json;
use tokio::sync::watch;
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
                let answer = answer_request(&state, &grant, text.as_str());
                if send_text(&mut socket, &answer.encode()).await.is_err() {
                    return;
                }
            }
        }
    }
}

fn answer_request(state: &ServerState, grant: &AgentGrant, frame: &str) -> Response {
    let Some(request) = Request::decode(frame) else {
        return Response::failure(
            0,
            ErrorCode::Invalid,
            "the frame is not a request with an id and a type",
        );
    };
    if !ALLOWED_REQUESTS.contains(&request.kind.as_str()) {
        // `{:?}` quotes and escapes, so an embedded newline cannot fake a log
        // line of its own.
        state.log.refusal(&format!(
            "agent {} asked for {:?}, which an agent may not",
            grant.agent_id,
            truncated(&request.kind, MAX_LOGGED_KIND_CHARS)
        ));
        return Response::failure(request.id, ErrorCode::Refused, NOT_ALLOWED);
    }
    match request.kind.as_str() {
        "agent.whoami" => {
            Response::success(request.id, json!({"agent_id": grant.agent_id.to_string()}))
        }
        other => Response::failure(
            request.id,
            ErrorCode::Refused,
            format!("{other} is not available to agents yet"),
        ),
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
