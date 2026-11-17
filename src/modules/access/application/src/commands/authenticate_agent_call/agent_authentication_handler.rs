// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use access_domain::value_objects::{Nonce, OfficeToken};
use kernel::AgentId;

use crate::common::errors::AccessError;
use crate::common::interfaces::{AgentCallStore, ProofScheme};

/// The proof of a connection of a running agent. The proof is the one of the
/// office token, keyed with the agent's own call credential (0002, plan D3).
/// The server's challenge comes first, before the client says which agent it
/// is, so the credential is looked up when the proof arrives.
#[derive(Clone)]
pub struct AgentAuthenticationHandler {
    credentials: Arc<dyn AgentCallStore>,
    proofs: Arc<dyn ProofScheme>,
    /// Checked against when the agent has no credential, so that an unknown
    /// agent and a wrong proof cost the same.
    decoy: OfficeToken,
}

/// What a connection that proved an agent's credential may do: act as that
/// agent while its session lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentGrant {
    pub agent_id: AgentId,
    pub session: String,
    /// The proof for `auth.ok`.
    pub server_proof: String,
}

impl AgentAuthenticationHandler {
    #[must_use]
    pub fn new(
        credentials: Arc<dyn AgentCallStore>,
        proofs: Arc<dyn ProofScheme>,
        decoy: OfficeToken,
    ) -> Self {
        Self {
            credentials,
            proofs,
            decoy,
        }
    }

    /// Checks the client proof of `agent_id` for `port` and makes the server
    /// proof.
    ///
    /// # Errors
    /// `ProofRejected` when the agent has no running session or the proof is
    /// wrong, with the same answer for both, so a caller cannot tell which
    /// agents run.
    pub fn complete(
        &self,
        nonce: &Nonce,
        agent_id: AgentId,
        port: u16,
        client_proof: &str,
    ) -> Result<AgentGrant, AccessError> {
        let credential = self.credentials.current(agent_id);
        let token = credential
            .as_ref()
            .map_or(&self.decoy, |credential| &credential.token);
        let proof_is_valid = self
            .proofs
            .client_proof_is_valid(token, nonce, port, client_proof);
        let Some(credential) = credential.filter(|_| proof_is_valid) else {
            return Err(AccessError::ProofRejected);
        };
        Ok(AgentGrant {
            agent_id,
            session: credential.session,
            server_proof: self.proofs.server_proof(&credential.token, nonce, port)?,
        })
    }

    /// Whether the session of a grant still runs. A connection of an agent
    /// asks this before it acts, so a revoked credential stops the
    /// connection, not only new ones.
    #[must_use]
    pub fn is_live(&self, grant: &AgentGrant) -> bool {
        self.credentials
            .current(grant.agent_id)
            .is_some_and(|now| now.session == grant.session)
    }
}
