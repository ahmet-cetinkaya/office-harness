// SPDX-License-Identifier: AGPL-3.0-or-later

use access_application::common::errors::AccessError;
use access_application::common::interfaces::ProofScheme;
use access_domain::value_objects::{Nonce, OfficeToken};
use wire::{ProofRole, compute_proof, proof_is_valid};

/// The proof scheme of the wire protocol: HMAC-SHA256 over the nonce, the
/// port, and the role.
#[derive(Debug, Clone, Copy, Default)]
pub struct HmacProofScheme;

impl ProofScheme for HmacProofScheme {
    fn client_proof_is_valid(
        &self,
        token: &OfficeToken,
        nonce: &Nonce,
        port: u16,
        proof: &str,
    ) -> bool {
        proof_is_valid(
            token.expose_secret(),
            nonce.as_str(),
            port,
            ProofRole::Client,
            proof,
        )
    }

    fn server_proof(
        &self,
        token: &OfficeToken,
        nonce: &Nonce,
        port: u16,
    ) -> Result<String, AccessError> {
        compute_proof(
            token.expose_secret(),
            nonce.as_str(),
            port,
            ProofRole::Server,
        )
        .map_err(|_| AccessError::ProofUnavailable)
    }
}
