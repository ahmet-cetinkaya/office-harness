// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::ControlFlow;
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket};
use tokio::sync::{broadcast, mpsc, watch};
use tokio::time::timeout;
use wire::{CLOSE_UNAUTHORIZED, ErrorCode, Request, Response};

use super::agent_requests::AgentGateway;
use super::handshake::{HandshakeFailure, authenticate};
use super::server_state::ServerState;
use super::terminal_requests::{
    ConnectionTerminals, FRAME_BACKLOG, TERMINAL_PREFIX, TerminalAnswer,
};

/// How many answers of agent requests may wait for one connection to send them.
const RESPONSE_BACKLOG: usize = 64;
const CLOSE_REPLY_WAIT: Duration = Duration::from_secs(1);
/// How long the close of a connection that fell behind may take in all.
const OVERFLOW_CLOSE_LIMIT: Duration = Duration::from_secs(2);
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

async fn send_binary(socket: &mut WebSocket, frame: Vec<u8>) -> Result<(), Disconnected> {
    socket
        .send(Message::Binary(frame.into()))
        .await
        .map_err(|_| Disconnected)
}

async fn run_session(socket: WebSocket, state: ServerState, generation: u64) {
    let (frames_sender, frames) = mpsc::channel(FRAME_BACKLOG);
    let mut terminals = ConnectionTerminals::new(frames_sender);
    // Agent requests can take long (a stop waits for its grace period), so
    // they run in their own tasks and answer through this channel. The loop
    // keeps serving terminal input and output meanwhile. Answers carry the
    // request id, so they may arrive out of order.
    let (responses_sender, responses) = mpsc::channel(RESPONSE_BACKLOG);
    let channels = SessionChannels {
        frames,
        responses,
        responses_sender,
    };
    session_loop(socket, &state, generation, channels, &mut terminals).await;
    state.terminals.release(terminals).await;
}

/// The channels that feed one connection's loop.
struct SessionChannels {
    frames: mpsc::Receiver<Vec<u8>>,
    responses: mpsc::Receiver<String>,
    responses_sender: mpsc::Sender<String>,
}

async fn session_loop(
    mut socket: WebSocket,
    state: &ServerState,
    generation: u64,
    channels: SessionChannels,
    terminals: &mut ConnectionTerminals,
) {
    let SessionChannels {
        mut frames,
        mut responses,
        responses_sender,
    } = channels;
    let mut events = state.events.subscribe();
    let token_replaced = token_replaced(state.token_generations.clone(), generation);
    let stopping = server_stopping(state.shutdown.clone());
    let overflow = terminals.overflow_signal();
    let overflowed = overflow.notified();
    tokio::pin!(token_replaced, stopping, overflowed);
    loop {
        tokio::select! {
            biased;
            () = &mut token_replaced => return close_with(&mut socket, CLOSE_UNAUTHORIZED).await,
            () = &mut stopping => return close_with(&mut socket, CLOSE_GOING_AWAY).await,
            () = &mut overflowed => return close_overflowed(&mut socket).await,
            event = events.recv() => {
                if forward_event(&mut socket, event).await.is_break() {
                    return;
                }
            }
            Some(frame) = frames.recv() => {
                // A client that stops reading blocks this send, and its
                // frames pile up behind it, so the overflow signal also ends
                // the wait.
                let sent = tokio::select! {
                    biased;
                    () = &mut overflowed => return close_overflowed(&mut socket).await,
                    sent = send_binary(&mut socket, frame) => sent,
                };
                if sent.is_err() {
                    return;
                }
            }
            Some(answer) = responses.recv() => {
                if send_text(&mut socket, &answer).await.is_err() {
                    return;
                }
            }
            incoming = socket.recv() => {
                if handle_incoming(&mut socket, state, terminals, &responses_sender, incoming).await.is_break() {
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
    terminals: &mut ConnectionTerminals,
    responses: &mpsc::Sender<String>,
    incoming: Option<Result<Message, axum::Error>>,
) -> ControlFlow<()> {
    match incoming {
        Some(Ok(Message::Text(text))) => {
            match answer_request(socket, state, terminals, responses, text.as_str()).await {
                Ok(()) => ControlFlow::Continue(()),
                Err(Disconnected) => ControlFlow::Break(()),
            }
        }
        Some(Ok(Message::Binary(_) | Message::Ping(_) | Message::Pong(_))) => {
            ControlFlow::Continue(())
        }
        Some(Ok(Message::Close(_)) | Err(_)) | None => ControlFlow::Break(()),
    }
}

async fn answer_request(
    socket: &mut WebSocket,
    state: &ServerState,
    terminals: &mut ConnectionTerminals,
    responses: &mpsc::Sender<String>,
    frame: &str,
) -> Result<(), Disconnected> {
    let answer = match Request::decode(frame) {
        Some(request) if request.kind.starts_with(TERMINAL_PREFIX) => {
            state.terminals.handle(&request, terminals).await
        }
        Some(request) if AgentGateway::accepts(&request.kind) => {
            spawn_agent_request(state.agents.clone(), request, responses.clone());
            return Ok(());
        }
        Some(request) => TerminalAnswer {
            response: state.dispatcher.dispatch(&request),
            snapshot_frame: None,
        },
        None => TerminalAnswer {
            response: Response::failure(
                UNKNOWN_REQUEST_ID,
                ErrorCode::Invalid,
                "the frame is not a request with an id and a type",
            ),
            snapshot_frame: None,
        },
    };
    send_text(socket, &answer.response.encode()).await?;
    match answer.snapshot_frame {
        Some(snapshot) => send_binary(socket, snapshot).await,
        None => Ok(()),
    }
}

/// Runs an agent request apart from the connection loop and queues its
/// encoded answer for the loop to send.
fn spawn_agent_request(gateway: AgentGateway, request: Request, responses: mpsc::Sender<String>) {
    tokio::spawn(async move {
        let answer = gateway.handle(request).await.encode();
        // The connection may be gone by now; the request still ran.
        let _ = responses.send(answer).await;
    });
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

/// Closes a connection that fell behind, with 1011, so the client reconnects
/// and attaches again. The peer may not read at all, so the close is bounded.
async fn close_overflowed(socket: &mut WebSocket) {
    let _ = timeout(
        OVERFLOW_CLOSE_LIMIT,
        close_with(socket, CLOSE_INTERNAL_ERROR),
    )
    .await;
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
