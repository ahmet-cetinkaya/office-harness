// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Map, Value};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;

use super::socket::Socket;
use crate::api_error::ApiError;

type Outcome = Result<Value, ApiError>;
type WaitingReplies = HashMap<u32, oneshot::Sender<Outcome>>;

/// A request from the webview, waiting for its turn on the socket.
pub struct PendingRequest {
    pub kind: String,
    pub fields: Map<String, Value>,
    pub reply: oneshot::Sender<Outcome>,
}

/// One frame from the server, sorted by what the core does with it.
#[derive(Debug, PartialEq)]
pub enum Inbound {
    Response { id: u32, outcome: Outcome },
    Event(Value),
    Ignored,
}

/// The request frame: the fields of the request, then the `id` and `type` that
/// the core controls, so a field cannot replace them.
pub fn encode_request(id: u32, kind: &str, fields: Map<String, Value>) -> String {
    let mut body = fields;
    body.insert("id".to_owned(), Value::from(id));
    body.insert("type".to_owned(), Value::from(kind));
    Value::Object(body).to_string()
}

pub fn classify_frame(text: &str) -> Inbound {
    let Ok(Value::Object(body)) = serde_json::from_str::<Value>(text) else {
        return Inbound::Ignored;
    };
    if let Some(id) = body.get("id").and_then(Value::as_u64) {
        let Ok(id) = u32::try_from(id) else {
            return Inbound::Ignored;
        };
        return Inbound::Response {
            id,
            outcome: outcome_of(&body),
        };
    }
    if body.contains_key("type") {
        Inbound::Event(Value::Object(body))
    } else {
        Inbound::Ignored
    }
}

fn outcome_of(body: &Map<String, Value>) -> Outcome {
    if body.get("ok").and_then(Value::as_bool) == Some(true) {
        return Ok(body.get("result").cloned().unwrap_or(Value::Null));
    }
    let error = body.get("error");
    let text = |key: &str| {
        error
            .and_then(|error| error.get(key))
            .and_then(Value::as_str)
            .unwrap_or_default()
    };
    Err(ApiError::new(text("code"), text("message")))
}

/// Serves the webview's requests and forwards the server's events until the
/// socket ends or the last request sender is dropped. Binary terminal frames
/// are not forwarded yet (T10 adds their channel).
pub async fn run_session(
    mut socket: Socket,
    mut requests: mpsc::Receiver<PendingRequest>,
    events: &broadcast::Sender<Value>,
) {
    let mut waiting = WaitingReplies::new();
    let mut next_id: u32 = 1;
    loop {
        tokio::select! {
            inbound = socket.next() => match inbound {
                Some(Ok(Message::Text(text))) => forward(text.as_str(), &mut waiting, events),
                Some(Ok(Message::Close(_) | Message::Frame(_)) | Err(_)) | None => return,
                Some(Ok(_)) => {}
            },
            request = requests.recv() => {
                let Some(request) = request else { return };
                let id = next_id;
                next_id = next_id.wrapping_add(1);
                let frame = encode_request(id, &request.kind, request.fields);
                if socket.send(Message::text(frame)).await.is_err() {
                    let _ = request.reply.send(Err(ApiError::not_connected()));
                    return;
                }
                waiting.insert(id, request.reply);
            }
        }
    }
}

fn forward(text: &str, waiting: &mut WaitingReplies, events: &broadcast::Sender<Value>) {
    match classify_frame(text) {
        Inbound::Response { id, outcome } => {
            if let Some(reply) = waiting.remove(&id) {
                let _ = reply.send(outcome);
            }
        }
        // No receiver just means that no webview listens yet.
        Inbound::Event(event) => {
            let _ = events.send(event);
        }
        Inbound::Ignored => {}
    }
}

#[cfg(test)]
#[path = "session.tests.rs"]
mod tests;
