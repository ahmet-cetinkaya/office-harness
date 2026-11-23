// SPDX-License-Identifier: AGPL-3.0-or-later

use jiff::tz::TimeZone;
use kernel::{ScheduleId, Timestamp};
use schedule_domain::{MAX_STEPS, OffReason, Repeat, Schedule, classify, due_times_in};

/// The longest time after a due time at which the office still starts the run.
pub const GRACE_MILLIS: i64 = 120_000;

/// A due time that runs now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DueRun {
    pub schedule_id: ScheduleId,
    pub due: Timestamp,
}

/// All the due times of one schedule that are found missed at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissedBatch {
    pub schedule_id: ScheduleId,
    pub count: u64,
    pub first: Timestamp,
    pub last: Timestamp,
}

/// What one round does: the runs to start, the missed batches to record, and
/// the schedules whose state changed (handled due, next due, switch, last
/// missed), already in their new state for the host to save before it starts
/// any run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Plan {
    pub runs: Vec<DueRun>,
    pub missed: Vec<MissedBatch>,
    pub changed: Vec<Schedule>,
}

/// Decides one round without touching a store or a clock.
pub struct RoundPlanner;

impl RoundPlanner {
    /// For each schedule that is on: its due times after the last handled due
    /// time up to `now` are runs (within the grace) or one missed batch. The
    /// handled due time moves to the last of them, so a clock set back runs
    /// nothing twice. A once schedule that has a run or a missed batch is
    /// switched off, with the off reason `TimeMissed` when it only missed. A
    /// due time before `started`, the moment the office began to run, is
    /// missed even within the grace (FR-005). A schedule whose next due time
    /// differs from the stored one (a zone change) is returned with the new
    /// one; a once schedule that has no due time left is switched off with
    /// the off reason `TimeMissed`.
    #[must_use]
    pub fn plan(
        now: Timestamp,
        started: Timestamp,
        zone: &TimeZone,
        schedules: &[Schedule],
    ) -> Plan {
        let mut plan = Plan::default();
        for schedule in schedules.iter().filter(|schedule| schedule.switched_on) {
            plan_one(now, started, zone, schedule, &mut plan);
        }
        plan
    }
}

fn plan_one(
    now: Timestamp,
    started: Timestamp,
    zone: &TimeZone,
    schedule: &Schedule,
    plan: &mut Plan,
) {
    let after = schedule.last_handled_due.unwrap_or(schedule.anchor);
    let window = due_times_in(
        &schedule.repeat,
        schedule.anchor,
        zone,
        after,
        now,
        MAX_STEPS,
    );
    let split = classify(&window, now, started, GRACE_MILLIS);

    let mut updated = schedule.clone();
    if let Some(last) = window.last() {
        updated.last_handled_due = Some(last);
    }
    for due in &split.runs {
        plan.runs.push(DueRun {
            schedule_id: schedule.id,
            due: *due,
        });
    }
    if let Some(missed) = split.missed {
        plan.missed.push(MissedBatch {
            schedule_id: schedule.id,
            count: missed.count,
            first: missed.first,
            last: missed.last,
        });
        updated.last_missed = Some(missed.last);
    }
    let was_handled = window.count > 0;
    if was_handled && matches!(schedule.repeat, Repeat::Once { .. }) {
        updated.switched_on = false;
        updated.off_reason = (split.runs.is_empty()).then_some(OffReason::TimeMissed);
    }
    updated.recompute_next_due(now, zone);
    // A zone change or a clock set back can leave a once schedule with a due
    // time that is already handled: it would stay on with nothing to run.
    if updated.switched_on && updated.next_due.is_none() {
        updated.switched_on = false;
        updated.off_reason = Some(OffReason::TimeMissed);
    }
    if updated != *schedule {
        plan.changed.push(updated);
    }
}

#[cfg(test)]
#[path = "round_planner.tests.rs"]
mod tests;
