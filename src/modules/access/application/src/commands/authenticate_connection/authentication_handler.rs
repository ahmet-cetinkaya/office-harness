// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use super::{ConnectionGrant, PendingAuthentication};
use crate::common::errors::AccessError;
use crate::common::interfaces::{NonceSource, ProofScheme, TokenStore};

/// The two steps of the mutual proof for one connection.
#[derive(Clone)]
pub struct AuthenticationHandler {
    store: Arc<dyn TokenStore>,
    nonces: Arc<dyn NonceSource>,
    proofs: Arc<dyn ProofScheme>,
}

impl AuthenticationHandler {
    #[must_use]
    pub fn new(
        store: Arc<dyn TokenStore>,
        nonces: Arc<dyn NonceSource>,
        proofs: Arc<dyn ProofScheme>,
    ) -> Self {
        Self {
            store,
            nonces,
            proofs,
        }
    }

    /// Step 1: a fresh nonce, bound to the token in force now.
    ///
    /// # Errors
    /// The store's or the nonce source's error.
    pub fn begin(&self) -> Result<PendingAuthentication, AccessError> {
        let snapshot = self.store.current()?;
        let nonce = self.nonces.next_nonce()?;
        Ok(PendingAuthentication { snapshot, nonce })
    }

    /// Step 2: checks the client proof for `port` and makes the server proof.
    ///
    /// # Errors
    /// `ProofRejected` when the client proof is wrong, whatever the reason.
    pub fn complete(
        &self,
        pending: &PendingAuthentication,
        port: u16,
        client_proof: &str,
    ) -> Result<ConnectionGrant, AccessError> {
        let token = &pending.snapshot.token;
        if !self
            .proofs
            .client_proof_is_valid(token, &pending.nonce, port, client_proof)
        {
            return Err(AccessError::ProofRejected);
        }
        Ok(ConnectionGrant {
            generation: pending.snapshot.generation,
            server_proof: self.proofs.server_proof(token, &pending.nonce, port)?,
        })
    }
}

#[cfg(test)]
#[path = "authentication_handler.tests.rs"]
mod tests;
