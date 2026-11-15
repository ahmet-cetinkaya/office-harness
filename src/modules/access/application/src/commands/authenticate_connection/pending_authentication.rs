// SPDX-License-Identifier: AGPL-3.0-or-later

use access_domain::value_objects::Nonce;

use crate::common::models::TokenSnapshot;

/// A challenge that was sent and waits for the client's proof. It holds the
/// token that was in force when the challenge was made.
pub struct PendingAuthentication {
    pub(super) snapshot: TokenSnapshot,
    pub(super) nonce: Nonce,
}

impl PendingAuthentication {
    /// The nonce to send in `auth.challenge`.
    #[must_use]
    pub fn nonce(&self) -> &Nonce {
        &self.nonce
    }
}
