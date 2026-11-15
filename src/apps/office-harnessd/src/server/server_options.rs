// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

/// How many connections may be in the handshake at once. Each can buffer up
/// to one maximum frame before it proves anything, so this bounds that memory.
pub const MAX_CONCURRENT_HANDSHAKES: usize = 8;

/// Server settings that tests change; the defaults follow the contract.
#[derive(Debug, Clone, Copy)]
pub struct ServerOptions {
    /// How long a new connection may take to send its proof.
    pub auth_timeout: Duration,
    /// A connection that finds no free handshake slot closes with 4401.
    pub max_concurrent_handshakes: usize,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            auth_timeout: wire::AUTH_TIMEOUT,
            max_concurrent_handshakes: MAX_CONCURRENT_HANDSHAKES,
        }
    }
}
