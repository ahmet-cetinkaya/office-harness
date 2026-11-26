// SPDX-License-Identifier: AGPL-3.0-or-later

use jiff::tz::TimeZone;

/// Where the machine's current time zone comes from. The host reads it each
/// round; tests pass a fixed zone or one that changes.
pub trait ZoneSource {
    fn current(&self) -> TimeZone;
}
