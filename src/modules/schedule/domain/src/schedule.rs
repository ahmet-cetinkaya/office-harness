// SPDX-License-Identifier: AGPL-3.0-or-later

use jiff::tz::TimeZone;
use kernel::{AgentId, ScheduleId, Timestamp};

use crate::Repeat;
use crate::due::next_due_after;

/// Why the office switched a schedule off by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffReason {
    TargetDeleted,
    TimeMissed,
}

/// A directive that the person set to go to one agent at due times.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    pub id: ScheduleId,
    pub name: String,
    pub text: String,
    pub target: AgentId,
    pub repeat: Repeat,
    pub switched_on: bool,
    /// The moment an every-N-hours repeat counts from: when the schedule was
    /// made or last edited.
    pub anchor: Timestamp,
    pub next_due: Option<Timestamp>,
    /// The latest due time with a recorded result; no due time at or before it
    /// is handled again, also after the clock is set back.
    pub last_handled_due: Option<Timestamp>,
    pub last_missed: Option<Timestamp>,
    pub off_reason: Option<OffReason>,
    /// A notification went out since the last successful run.
    pub notified_since_success: bool,
}

impl Schedule {
    /// The first due time later than both `now` and the last handled due time.
    #[must_use]
    pub fn next_due(&self, now: Timestamp, zone: &TimeZone) -> Option<Timestamp> {
        let after = self
            .last_handled_due
            .map_or(now, |handled| handled.max(now));
        next_due_after(&self.repeat, self.anchor, zone, after)
    }

    /// Sets `next_due` for the zone and time given: none while the schedule is
    /// off.
    pub fn recompute_next_due(&mut self, now: Timestamp, zone: &TimeZone) {
        self.next_due = self.switched_on.then(|| self.next_due(now, zone)).flatten();
    }

    /// Marks every due time up to `now` as handled, so a new schedule, an
    /// edit, or a switch on neither runs nor misses a time before this moment.
    pub fn mark_handled_through(&mut self, now: Timestamp) {
        self.last_handled_due = Some(
            self.last_handled_due
                .map_or(now, |handled| handled.max(now)),
        );
    }
}

#[cfg(test)]
#[path = "schedule.tests.rs"]
mod tests;
