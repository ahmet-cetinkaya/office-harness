// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

use crate::auth_message::AuthMessage;
use crate::proof::{ProofRole, compute_proof, nonce_is_well_formed, proof_is_valid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum HandshakeError {
    #[error("the server did not start with a challenge")]
    NoChallenge,
    #[error("the server sent no valid proof, so it is not the office server")]
    ServerNotProven,
    #[error("the token cannot be used")]
    UnusableToken,
}

/// The client half of the handshake, free of any socket. The caller sends
/// nothing but the frame from `answer_challenge` until `confirm_server`
/// succeeds, and closes the connection when either step fails.
pub struct ClientHandshake {
    token: String,
    port: u16,
    nonce: Option<String>,
    /// Set when an agent calls the office with its own credential.
    agent_id: Option<String>,
}

impl ClientHandshake {
    /// `port` is the one from `server.json`.
    #[must_use]
    pub fn new(token: &str, port: u16) -> Self {
        Self {
            token: token.to_owned(),
            port,
            nonce: None,
            agent_id: None,
        }
    }

    /// The handshake of an agent that calls from inside its session: the
    /// proof is keyed with the agent's call credential, and the `auth` frame
    /// names the agent.
    #[must_use]
    pub fn for_agent(agent_id: &str, credential: &str, port: u16) -> Self {
        Self {
            agent_id: Some(agent_id.to_owned()),
            ..Self::new(credential, port)
        }
    }

    /// Takes the server's first frame and returns the `auth` frame to send.
    ///
    /// # Errors
    /// `NoChallenge` when the frame is not a challenge.
    pub fn answer_challenge(&mut self, frame: &str) -> Result<String, HandshakeError> {
        let Some(AuthMessage::Challenge { nonce }) = AuthMessage::decode(frame) else {
            return Err(HandshakeError::NoChallenge);
        };
        if !nonce_is_well_formed(&nonce) {
            return Err(HandshakeError::NoChallenge);
        }
        let proof = compute_proof(&self.token, &nonce, self.port, ProofRole::Client)
            .map_err(|_| HandshakeError::UnusableToken)?;
        self.nonce = Some(nonce);
        Ok(AuthMessage::Auth {
            proof,
            agent_id: self.agent_id.clone(),
        }
        .encode())
    }

    /// Checks the server's `auth.ok` frame.
    ///
    /// # Errors
    /// `ServerNotProven` when the frame is not `auth.ok` or its proof is wrong,
    /// and when no challenge was answered first.
    pub fn confirm_server(&self, frame: &str) -> Result<(), HandshakeError> {
        let (Some(nonce), Some(AuthMessage::AuthOk { proof })) =
            (&self.nonce, AuthMessage::decode(frame))
        else {
            return Err(HandshakeError::ServerNotProven);
        };
        if proof_is_valid(&self.token, nonce, self.port, ProofRole::Server, &proof) {
            Ok(())
        } else {
            Err(HandshakeError::ServerNotProven)
        }
    }
}

#[cfg(test)]
#[path = "handshake.tests.rs"]
mod tests;
