// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use crate::bridges::WORK_BOUND_EXTRA;

/// How many connections may be in the handshake at once. Each can buffer up
/// to one maximum frame before it proves anything, so this bounds that memory.
pub const MAX_CONCURRENT_HANDSHAKES: usize = 8;

/// How long a bulk resume watches each start for an end by itself (0013 A2).
pub const RESUME_CHECK_PERIOD: Duration = Duration::from_secs(20);

/// How often the idle sleep looks at the agents. Four looks a minute keep
/// a sleep within 60 seconds of the idle time (0018/FR-002).
pub const IDLE_SLEEP_TICK: Duration = Duration::from_secs(15);

/// The length of one minute of the idle time setting (0018/FR-001).
pub const IDLE_SLEEP_MINUTE: Duration = Duration::from_secs(60);

/// How often the temp watcher looks at the temps (0020 D8).
pub const TEMP_TICK: Duration = Duration::from_secs(2);

/// The length of one minute of the temp idle limit (0020 D8).
pub const TEMP_MINUTE: Duration = Duration::from_secs(60);

/// Server settings that tests change; the defaults follow the contract.
#[derive(Debug, Clone, Copy)]
pub struct ServerOptions {
    /// How long a new connection may take to send its proof.
    pub auth_timeout: Duration,
    /// A connection that finds no free handshake slot closes with 1013 (Try
    /// Again Later).
    pub max_concurrent_handshakes: usize,
    /// The check period of a bulk resume and of a wake; tests shorten it.
    pub resume_check_period: Duration,
    /// How often the idle sleep looks at the agents; tests shorten it.
    pub idle_sleep_tick: Duration,
    /// One minute of the idle time setting; tests shorten it so a 5-minute
    /// idle time passes in a test.
    pub idle_sleep_minute: Duration,
    /// How often the temp watcher looks at the temps; tests shorten it.
    pub temp_tick: Duration,
    /// One minute of the temp idle limit; tests shorten it.
    pub temp_minute: Duration,
    /// The time added to the quiet period to make the work bound (0022 D3);
    /// tests shorten it.
    pub work_bound_extra: Duration,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            auth_timeout: wire::AUTH_TIMEOUT,
            max_concurrent_handshakes: MAX_CONCURRENT_HANDSHAKES,
            resume_check_period: RESUME_CHECK_PERIOD,
            idle_sleep_tick: IDLE_SLEEP_TICK,
            idle_sleep_minute: IDLE_SLEEP_MINUTE,
            temp_tick: TEMP_TICK,
            temp_minute: TEMP_MINUTE,
            work_bound_extra: WORK_BOUND_EXTRA,
        }
    }
}
