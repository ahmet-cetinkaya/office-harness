// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, WebSocketConfig};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async_with_config};
use wire::{
    CLOSE_TRY_AGAIN_LATER, CLOSE_UNAUTHORIZED, ClientHandshake, HandshakeError, MAX_FRAME_BYTES,
    WS_PATH,
};

use super::state_tracker::Failure;

pub type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// The server gives up on a silent client after 5 seconds; the client waits a
/// little longer than that for the server's side of the proof.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Connects to the server on loopback and runs the mutual proof. Only a socket
/// whose server proved that it knows the token is returned, so nothing but the
/// proof ever travels to a server that has not proven itself.
pub async fn open_authenticated(port: u16, token: &str) -> Result<Socket, Failure> {
    let mut socket = timeout(CONNECT_TIMEOUT, connect(port))
        .await
        .map_err(|_elapsed| Failure::NotReachable)??;
    timeout(HANDSHAKE_TIMEOUT, prove(&mut socket, token, port))
        .await
        .map_err(|_elapsed| Failure::NotReachable)??;
    Ok(socket)
}

async fn connect(port: u16) -> Result<Socket, Failure> {
    let address = format!("ws://127.0.0.1:{port}{WS_PATH}");
    let configuration = WebSocketConfig::default()
        .max_message_size(Some(MAX_FRAME_BYTES))
        .max_frame_size(Some(MAX_FRAME_BYTES));
    connect_async_with_config(address, Some(configuration), false)
        .await
        .map(|(socket, _response)| socket)
        .map_err(|_| Failure::NotReachable)
}

async fn prove(socket: &mut Socket, token: &str, port: u16) -> Result<(), Failure> {
    let mut handshake = ClientHandshake::new(token, port);
    let challenge = next_text(socket).await.map_err(from_unproven_server)?;
    let answer = handshake
        .answer_challenge(&challenge)
        .map_err(|error| match error {
            HandshakeError::UnusableToken => Failure::Rejected,
            HandshakeError::NoChallenge | HandshakeError::ServerNotProven => Failure::NotReachable,
        })?;
    socket
        .send(Message::text(answer))
        .await
        .map_err(|_| Failure::NotReachable)?;
    let confirmation = next_text(socket).await?;
    handshake
        .confirm_server(&confirmation)
        .map_err(|_| Failure::NotReachable)
}

/// A server that has not answered the challenge has proven nothing, so its
/// unauthorized close says nothing about the token.
fn from_unproven_server(failure: Failure) -> Failure {
    match failure {
        Failure::Rejected => Failure::NotReachable,
        other => other,
    }
}

async fn next_text(socket: &mut Socket) -> Result<String, Failure> {
    loop {
        match socket.next().await {
            Some(Ok(Message::Text(text))) => return Ok(text.as_str().to_owned()),
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
            Some(Ok(Message::Close(frame))) => return Err(failure_of_close(frame.as_ref())),
            Some(Ok(_) | Err(_)) | None => return Err(Failure::NotReachable),
        }
    }
}

fn failure_of_close(frame: Option<&CloseFrame>) -> Failure {
    match frame.map(|frame| u16::from(frame.code)) {
        Some(CLOSE_UNAUTHORIZED) => Failure::Rejected,
        Some(CLOSE_TRY_AGAIN_LATER) => Failure::Busy,
        _ => Failure::NotReachable,
    }
}

#[cfg(test)]
#[path = "socket.tests.rs"]
mod tests;
