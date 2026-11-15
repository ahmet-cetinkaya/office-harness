// SPDX-License-Identifier: AGPL-3.0-or-later

use access_domain::value_objects::Nonce;

use crate::common::errors::AccessError;

/// Where fresh challenges come from. Each call returns a new nonce.
pub trait NonceSource: Send + Sync {
    /// # Errors
    /// `Randomness` when the system has no random bytes.
    fn next_nonce(&self) -> Result<Nonce, AccessError>;
}
