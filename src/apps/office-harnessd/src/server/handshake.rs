// SPDX-License-Identifier: AGPL-3.0-or-later

use access_application::commands::authenticate_connection::ConnectionGrant;
use axum::extract::ws::{Message, WebSocket};
use tokio::time::timeout;
use wire::{AuthMessage, PRE_AUTH_FRAME_LIMIT};

use super::connection::{Disconnected, send_text};
use super::server_state::ServerState;

/// How the handshake ended without a grant.
#[derive(Debug, PartialEq, Eq)]
pub enum HandshakeFailure {
    /// Wrong, missing, late, or oversized proof, or any other first frame.
    Rejected,
    /// The server could not run the handshake.
    Unavailable,
    /// The peer left, so there is nobody to close.
    Disconnected,
}

impl From<Disconnected> for HandshakeFailure {
    fn from(_: Disconnected) -> Self {
        Self::Disconnected
    }
}

/// Runs the server side of the mutual proof (contract, Authentication). Until
/// `auth.ok` the server sends the challenge and nothing else.
pub async fn authenticate(
    socket: &mut WebSocket,
    state: &ServerState,
) -> Result<ConnectionGrant, HandshakeFailure> {
    let pending = state.authentication.begin().map_err(|error| {
        state
            .log
            .error(&format!("cannot start authentication: {error}"));
        HandshakeFailure::Unavailable
    })?;
    let challenge = AuthMessage::Challenge {
        nonce: pending.nonce().as_str().to_owned(),
    };
    send_text(socket, &challenge.encode()).await?;

    let received = timeout(state.options.auth_timeout, receive_client_proof(socket))
        .await
        .map_err(|_elapsed| HandshakeFailure::Rejected);
    let proof = received
        .and_then(|outcome| outcome)
        .inspect_err(|failure| {
            state
                .log
                .info(&format!("connection refused before auth.ok: {failure:?}"));
        })?;

    let grant = state
        .authentication
        .complete(&pending, state.port, &proof)
        .map_err(|_| {
            state.log.info("connection refused: the proof is not valid");
            HandshakeFailure::Rejected
        })?;
    let accepted = AuthMessage::AuthOk {
        proof: grant.server_proof.clone(),
    };
    send_text(socket, &accepted.encode()).await?;
    Ok(grant)
}

async fn receive_client_proof(socket: &mut WebSocket) -> Result<String, HandshakeFailure> {
    match socket.recv().await {
        Some(Ok(Message::Text(text))) => proof_from_frame(text.as_str()),
        Some(Ok(Message::Close(_))) | None => Err(HandshakeFailure::Disconnected),
        // A frame that breaks the protocol, such as one over the size limit,
        // surfaces as an error; it is a refusal like any other bad frame.
        Some(Ok(_) | Err(_)) => Err(HandshakeFailure::Rejected),
    }
}

fn proof_from_frame(text: &str) -> Result<String, HandshakeFailure> {
    if text.len() > PRE_AUTH_FRAME_LIMIT {
        return Err(HandshakeFailure::Rejected);
    }
    match AuthMessage::decode(text) {
        Some(AuthMessage::Auth { proof }) => Ok(proof),
        _ => Err(HandshakeFailure::Rejected),
    }
}
