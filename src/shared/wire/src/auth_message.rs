// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::Deserialize;
use serde_json::json;

/// The three frames of the handshake. The token itself never appears in one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type")]
pub enum AuthMessage {
    /// Server to client, the first and only frame before the client proof.
    #[serde(rename = "auth.challenge")]
    Challenge { nonce: String },
    /// Client to server.
    #[serde(rename = "auth")]
    Auth {
        proof: String,
        /// Set by an agent that calls the office from inside its session;
        /// the proof is then keyed with that agent's call credential.
        #[serde(default)]
        agent_id: Option<String>,
    },
    /// Server to client, carrying the server's own proof.
    #[serde(rename = "auth.ok")]
    AuthOk { proof: String },
}

impl AuthMessage {
    /// The JSON text frame.
    #[must_use]
    pub fn encode(&self) -> String {
        match self {
            Self::Challenge { nonce } => json!({"type": "auth.challenge", "nonce": nonce}),
            Self::Auth {
                proof,
                agent_id: None,
            } => json!({"type": "auth", "proof": proof}),
            Self::Auth {
                proof,
                agent_id: Some(agent_id),
            } => json!({"type": "auth", "proof": proof, "agent_id": agent_id}),
            Self::AuthOk { proof } => json!({"type": "auth.ok", "proof": proof}),
        }
        .to_string()
    }

    /// Reads a text frame; anything that is not one of the three is `None`.
    #[must_use]
    pub fn decode(frame: &str) -> Option<Self> {
        serde_json::from_str(frame).ok()
    }
}

#[cfg(test)]
#[path = "auth_message.tests.rs"]
mod tests;
