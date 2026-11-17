// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::{SystemTime, UNIX_EPOCH};

use agent_application::common::interfaces::Clock;
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

fn wall_now() -> Timestamp {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    Timestamp::from_unix_millis(i64::try_from(since_epoch.as_millis()).unwrap_or(i64::MAX))
}
