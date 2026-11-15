// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::ControlFlow;
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket};
use tokio::sync::{broadcast, watch};
use tokio::time::timeout;
use wire::{CLOSE_UNAUTHORIZED, ErrorCode, Request, Response};

use super::handshake::{HandshakeFailure, authenticate};
use super::server_state::ServerState;

const CLOSE_REPLY_WAIT: Duration = Duration::from_secs(1);
const CLOSE_GOING_AWAY: u16 = 1001;
const CLOSE_INTERNAL_ERROR: u16 = 1011;
/// The `id` of the answer to a frame that has none.
const UNKNOWN_REQUEST_ID: u32 = 0;

/// The peer is gone: a send failed or the stream ended.
#[derive(Debug, PartialEq, Eq)]
pub struct Disconnected;

/// One WebSocket from the upgrade to its end: the handshake, then requests
/// and events until the client leaves, the token is replaced, or the server
/// stops.
pub async fn serve_connection(mut socket: WebSocket, state: ServerState) {
    let outcome = {
        let Ok(_permit) = state.handshake_permits.try_acquire() else {
            state
                .log
                .info("connection refused: too many handshakes at once");
            return close_with(&mut socket, CLOSE_UNAUTHORIZED).await;
        };
        authenticate(&mut socket, &state).await
    };
    match outcome {
        Ok(grant) => run_session(socket, state, grant.generation).await,
        Err(HandshakeFailure::Rejected) => close_with(&mut socket, CLOSE_UNAUTHORIZED).await,
        Err(HandshakeFailure::Unavailable) => close_with(&mut socket, CLOSE_INTERNAL_ERROR).await,
        Err(HandshakeFailure::Disconnected) => {}
    }
}

pub async fn send_text(socket: &mut WebSocket, text: &str) -> Result<(), Disconnected> {
    socket
        .send(Message::Text(text.into()))
        .await
        .map_err(|_| Disconnected)
}

async fn run_session(mut socket: WebSocket, state: ServerState, generation: u64) {
    let mut events = state.events.subscribe();
    let token_replaced = token_replaced(state.token_generations.clone(), generation);
    let stopping = server_stopping(state.shutdown.clone());
    tokio::pin!(token_replaced, stopping);
    loop {
        tokio::select! {
            biased;
            () = &mut token_replaced => return close_with(&mut socket, CLOSE_UNAUTHORIZED).await,
            () = &mut stopping => return close_with(&mut socket, CLOSE_GOING_AWAY).await,
            event = events.recv() => {
                if forward_event(&mut socket, event).await.is_break() {
                    return;
                }
            }
            incoming = socket.recv() => {
                if handle_incoming(&mut socket, &state, incoming).await.is_break() {
                    return;
                }
            }
        }
    }
}

async fn forward_event(
    socket: &mut WebSocket,
    event: Result<std::sync::Arc<str>, broadcast::error::RecvError>,
) -> ControlFlow<()> {
    match event {
        Ok(frame) => match send_text(socket, &frame).await {
            Ok(()) => ControlFlow::Continue(()),
            Err(Disconnected) => ControlFlow::Break(()),
        },
        // A slow connection skips the events it missed and keeps going.
        Err(broadcast::error::RecvError::Lagged(_)) => ControlFlow::Continue(()),
        Err(broadcast::error::RecvError::Closed) => ControlFlow::Break(()),
    }
}

async fn handle_incoming(
    socket: &mut WebSocket,
    state: &ServerState,
    incoming: Option<Result<Message, axum::Error>>,
) -> ControlFlow<()> {
    match incoming {
        Some(Ok(Message::Text(text))) => match answer_request(socket, state, text.as_str()).await {
            Ok(()) => ControlFlow::Continue(()),
            Err(Disconnected) => ControlFlow::Break(()),
        },
        Some(Ok(Message::Binary(_) | Message::Ping(_) | Message::Pong(_))) => {
            ControlFlow::Continue(())
        }
        Some(Ok(Message::Close(_)) | Err(_)) | None => ControlFlow::Break(()),
    }
}

async fn answer_request(
    socket: &mut WebSocket,
    state: &ServerState,
    frame: &str,
) -> Result<(), Disconnected> {
    let response = match Request::decode(frame) {
        Some(request) => state.dispatcher.dispatch(&request),
        None => Response::failure(
            UNKNOWN_REQUEST_ID,
            ErrorCode::Invalid,
            "the frame is not a request with an id and a type",
        ),
    };
    send_text(socket, &response.encode()).await
}

/// Resolves when a newer token generation than `generation` exists. Checking
/// the value first covers a regeneration that happened before the watch.
async fn token_replaced(mut generations: watch::Receiver<u64>, generation: u64) {
    loop {
        if *generations.borrow_and_update() > generation {
            return;
        }
        if generations.changed().await.is_err() {
            return;
        }
    }
}

async fn server_stopping(mut shutdown: watch::Receiver<bool>) {
    // A dropped sender also means the server is going away.
    let _ = shutdown.wait_for(|stopping| *stopping).await;
}

async fn close_with(socket: &mut WebSocket, code: u16) {
    let frame = CloseFrame {
        code,
        reason: Utf8Bytes::default(),
    };
    if socket.send(Message::Close(Some(frame))).await.is_err() {
        return;
    }
    // Waiting for the peer's close frame lets the closing handshake finish
    // before the socket drops, so the peer reads the code. A peer that never
    // answers cannot hold the task past the wait.
    let _ = timeout(CLOSE_REPLY_WAIT, wait_for_peer_close(socket)).await;
}

async fn wait_for_peer_close(socket: &mut WebSocket) {
    while let Some(Ok(message)) = socket.recv().await {
        if matches!(message, Message::Close(_)) {
            return;
        }
    }
}
