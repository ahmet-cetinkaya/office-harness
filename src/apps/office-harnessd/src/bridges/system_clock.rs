// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use agent_application::common::interfaces::Clock;
use hierarchy_application::common::ports::HoldClock;
use kernel::Timestamp;
use notification_application::common::interfaces::NotificationClock;

/// The wall clock.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        wall_now()
    }
}

impl NotificationClock for SystemClock {
    fn now(&self) -> Timestamp {
        wall_now()
    }
}

/// The hold service's two times: the wall clock for a hold's `since`, and
/// for the rate windows the wall time read once, then moved on by the
/// monotonic clock, so a step of the system clock neither stretches nor
/// shortens a window (0007).
pub struct SteadyClock {
    started_at: Timestamp,
    started: Instant,
}

impl Default for SteadyClock {
    fn default() -> Self {
        Self {
            started_at: wall_now(),
            started: Instant::now(),
        }
    }
}

impl HoldClock for SteadyClock {
    fn now(&self) -> Timestamp {
        wall_now()
    }

    fn steady_now(&self) -> Timestamp {
        let elapsed = i64::try_from(self.started.elapsed().as_millis()).unwrap_or(i64::MAX);
        Timestamp::from_unix_millis(self.started_at.as_unix_millis().saturating_add(elapsed))
    }
}

fn wall_now() -> Timestamp {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    Timestamp::from_unix_millis(i64::try_from(since_epoch.as_millis()).unwrap_or(i64::MAX))
}
