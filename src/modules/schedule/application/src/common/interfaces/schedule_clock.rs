// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::Timestamp;

/// The wall clock, as the template module's clock.
pub trait ScheduleClock: Send + Sync {
    fn now(&self) -> Timestamp;
}
