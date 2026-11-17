// SPDX-License-Identifier: AGPL-3.0-or-later

use access_application::commands::authenticate_agent_call::AgentGrant;
use access_application::commands::authenticate_connection::ConnectionGrant;
use axum::extract::ws::{Message, WebSocket};
use kernel::AgentId;
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

/// Who the connection proved to be: the desktop app with the office token, or
/// a running agent with its call credential (0002).
#[derive(Debug)]
pub enum Authenticated {
    Office(ConnectionGrant),
    Agent(AgentGrant),
}

/// What the client's `auth` frame carried.
struct ClientAuth {
    proof: String,
    agent_id: Option<String>,
}

/// Runs the server side of the mutual proof (contract, Authentication). Until
/// `auth.ok` the server sends the challenge and nothing else.
pub async fn authenticate(
    socket: &mut WebSocket,
    state: &ServerState,
) -> Result<Authenticated, HandshakeFailure> {
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
    let client = received
        .and_then(|outcome| outcome)
        .inspect_err(|failure| {
            state
                .log
                .refusal(&format!("connection refused before auth.ok: {failure:?}"));
        })?;

    let authenticated = match client.agent_id.as_deref() {
        None => state
            .authentication
            .complete(&pending, state.port, &client.proof)
            .map(Authenticated::Office),
        Some(agent_id) => complete_agent(state, &pending, agent_id, &client.proof),
    }
    .map_err(|_| {
        state
            .log
            .refusal("connection refused: the proof is not valid");
        HandshakeFailure::Rejected
    })?;
    let server_proof = match &authenticated {
        Authenticated::Office(grant) => grant.server_proof.clone(),
        Authenticated::Agent(grant) => grant.server_proof.clone(),
    };
    send_text(
        socket,
        &AuthMessage::AuthOk {
            proof: server_proof,
        }
        .encode(),
    )
    .await?;
    Ok(authenticated)
}

/// The agent path of the proof. A malformed agent id is the same refusal as a
/// wrong proof.
fn complete_agent(
    state: &ServerState,
    pending: &access_application::commands::authenticate_connection::PendingAuthentication,
    agent_id: &str,
    proof: &str,
) -> Result<Authenticated, access_application::common::errors::AccessError> {
    let agent_id = agent_id
        .parse::<AgentId>()
        .map_err(|_| access_application::common::errors::AccessError::ProofRejected)?;
    state
        .agent_authentication
        .complete(pending.nonce(), agent_id, state.port, proof)
        .map(Authenticated::Agent)
}

async fn receive_client_proof(socket: &mut WebSocket) -> Result<ClientAuth, HandshakeFailure> {
    match socket.recv().await {
        Some(Ok(Message::Text(text))) => proof_from_frame(text.as_str()),
        Some(Ok(Message::Close(_))) | None => Err(HandshakeFailure::Disconnected),
        // A frame that breaks the protocol, such as one over the size limit,
        // surfaces as an error; it is a refusal like any other bad frame.
        Some(Ok(_) | Err(_)) => Err(HandshakeFailure::Rejected),
    }
}

fn proof_from_frame(text: &str) -> Result<ClientAuth, HandshakeFailure> {
    if text.len() > PRE_AUTH_FRAME_LIMIT {
        return Err(HandshakeFailure::Rejected);
    }
    match AuthMessage::decode(text) {
        Some(AuthMessage::Auth { proof, agent_id }) => Ok(ClientAuth { proof, agent_id }),
        _ => Err(HandshakeFailure::Rejected),
    }
}
