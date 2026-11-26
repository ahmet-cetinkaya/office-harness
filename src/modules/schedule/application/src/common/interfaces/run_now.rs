// SPDX-License-Identifier: AGPL-3.0-or-later

use schedule_domain::Schedule;

/// Makes one run of a schedule at the person's request, with the same guards
/// as a run at a due time. The host implements it; it changes neither the next
/// due time nor the switch.
pub trait RunNow: Send + Sync {
    /// # Errors
    /// The reason the run is refused before it starts.
    fn run_now(&self, schedule: &Schedule) -> Result<(), String>;
}
