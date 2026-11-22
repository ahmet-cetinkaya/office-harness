// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

/// How many connections may be in the handshake at once. Each can buffer up
/// to one maximum frame before it proves anything, so this bounds that memory.
pub const MAX_CONCURRENT_HANDSHAKES: usize = 8;

/// How long a bulk resume watches each start for an end by itself (0013 A2).
pub const RESUME_CHECK_PERIOD: Duration = Duration::from_secs(20);

/// Server settings that tests change; the defaults follow the contract.
#[derive(Debug, Clone, Copy)]
pub struct ServerOptions {
    /// How long a new connection may take to send its proof.
    pub auth_timeout: Duration,
    /// A connection that finds no free handshake slot closes with 1013 (Try
    /// Again Later).
    pub max_concurrent_handshakes: usize,
    /// The check period of a bulk resume; tests shorten it.
    pub resume_check_period: Duration,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            auth_timeout: wire::AUTH_TIMEOUT,
            max_concurrent_handshakes: MAX_CONCURRENT_HANDSHAKES,
            resume_check_period: RESUME_CHECK_PERIOD,
        }
    }
}
