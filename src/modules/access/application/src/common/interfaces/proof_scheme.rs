// SPDX-License-Identifier: AGPL-3.0-or-later

use access_domain::value_objects::{Nonce, OfficeToken};

use crate::common::errors::AccessError;

/// How a proof of the token is made and checked. The wire protocol defines
/// the scheme; this port keeps the use case free of it.
pub trait ProofScheme: Send + Sync {
    /// Checks the client's proof in constant time.
    fn client_proof_is_valid(
        &self,
        token: &OfficeToken,
        nonce: &Nonce,
        port: u16,
        proof: &str,
    ) -> bool;

    /// The proof that the server knows the token.
    ///
    /// # Errors
    /// `ProofUnavailable` when the proof cannot be computed.
    fn server_proof(
        &self,
        token: &OfficeToken,
        nonce: &Nonce,
        port: u16,
    ) -> Result<String, AccessError>;
}
