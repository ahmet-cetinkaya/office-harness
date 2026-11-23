// SPDX-License-Identifier: AGPL-3.0-or-later

use schedule_domain::ScheduleCap;

/// The schedule cap of the office settings. The host reads the stored value
/// at each save, so a changed cap applies to the next save.
pub trait ScheduleCapSource: Send + Sync {
    fn cap(&self) -> ScheduleCap;
}
