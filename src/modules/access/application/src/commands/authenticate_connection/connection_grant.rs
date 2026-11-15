// SPDX-License-Identifier: AGPL-3.0-or-later

/// The result of a proof that was right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionGrant {
    /// The token generation that the connection authenticated with.
    pub generation: u64,
    /// The proof for `auth.ok`.
    pub server_proof: String,
}
