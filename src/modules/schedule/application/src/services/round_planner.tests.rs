// SPDX-License-Identifier: AGPL-3.0-or-later

use jiff::civil::{date, time};
use kernel::AgentId;

use super::*;
use crate::test_support::{HOUR, NOW};

fn at(millis: i64) -> Timestamp {
    Timestamp::from_unix_millis(millis)
}

fn schedule(repeat: Repeat, handled_through: i64) -> Schedule {
    let mut schedule = Schedule {
        id: ScheduleId::from_u128(1),
        name: "Nightly".to_owned(),
        text: "go".to_owned(),
        target: AgentId::from_u128(2),
        repeat,
        switched_on: true,
        anchor: at(NOW),
        next_due: None,
        last_handled_due: Some(at(handled_through)),
        last_missed: None,
        off_reason: None,
        notified_since_success: false,
    };
    schedule.recompute_next_due(at(handled_through), &TimeZone::UTC);
    schedule
}

fn hourly() -> Repeat {
    Repeat::EveryHours { hours: 1 }
}

#[test]
fn nothing_is_due_before_the_next_due_time() {
    let schedule = schedule(hourly(), NOW);
    let plan = RoundPlanner::plan(at(NOW + HOUR - 1), at(NOW), &TimeZone::UTC, &[schedule]);
    assert_eq!(plan, Plan::default());
}

#[test]
fn a_due_time_within_the_grace_is_a_run() {
    let schedule = schedule(hourly(), NOW);
    let now = at(NOW + HOUR + GRACE_MILLIS);
    let plan = RoundPlanner::plan(
        now,
        at(NOW),
        &TimeZone::UTC,
        std::slice::from_ref(&schedule),
    );
    assert_eq!(
        plan.runs,
        [DueRun {
            schedule_id: schedule.id,
            due: at(NOW + HOUR)
        }]
    );
    assert_eq!(plan.missed, []);
    assert_eq!(plan.changed[0].last_handled_due, Some(at(NOW + HOUR)));
    assert_eq!(plan.changed[0].next_due, Some(at(NOW + 2 * HOUR)));
}

#[test]
fn a_due_time_past_the_grace_is_missed() {
    let schedule = schedule(hourly(), NOW);
    let now = at(NOW + HOUR + GRACE_MILLIS + 1);
    let plan = RoundPlanner::plan(now, at(NOW), &TimeZone::UTC, &[schedule]);
    assert_eq!(plan.runs, []);
    assert_eq!(plan.missed.len(), 1);
    assert_eq!(plan.missed[0].count, 1);
}

#[test]
fn three_missed_hourly_due_times_make_one_batch() {
    let schedule = schedule(hourly(), NOW);
    let now = at(NOW + 3 * HOUR + 30 * 60_000);
    let plan = RoundPlanner::plan(
        now,
        at(NOW),
        &TimeZone::UTC,
        std::slice::from_ref(&schedule),
    );
    assert_eq!(plan.runs, []);
    assert_eq!(
        plan.missed,
        [MissedBatch {
            schedule_id: schedule.id,
            count: 3,
            first: at(NOW + HOUR),
            last: at(NOW + 3 * HOUR)
        }]
    );
    let changed = &plan.changed[0];
    assert_eq!(changed.last_handled_due, Some(at(NOW + 3 * HOUR)));
    assert_eq!(changed.last_missed, Some(at(NOW + 3 * HOUR)));
    assert_eq!(changed.next_due, Some(at(NOW + 4 * HOUR)));
    assert!(changed.switched_on);
}

#[test]
fn missed_and_fresh_due_times_split_in_one_round() {
    let schedule = schedule(hourly(), NOW);
    let now = at(NOW + 3 * HOUR + 60_000);
    let plan = RoundPlanner::plan(now, at(NOW), &TimeZone::UTC, &[schedule]);
    assert_eq!(plan.runs.len(), 1);
    assert_eq!(plan.missed[0].count, 2);
    assert_eq!(plan.missed[0].last, at(NOW + 2 * HOUR));
}

#[test]
fn a_once_schedule_that_missed_is_switched_off_with_the_time_missed_reason() {
    let once = Repeat::Once {
        date: date(2026, 6, 1),
        time: time(13, 0, 0, 0),
    };
    let schedule = schedule(once, NOW);
    let now = at(NOW + 5 * HOUR);
    let plan = RoundPlanner::plan(now, at(NOW), &TimeZone::UTC, &[schedule]);
    assert_eq!(plan.missed.len(), 1);
    let changed = &plan.changed[0];
    assert!(!changed.switched_on);
    assert_eq!(changed.off_reason, Some(OffReason::TimeMissed));
    assert_eq!(changed.next_due, None);
}

#[test]
fn a_once_schedule_that_ran_is_switched_off_without_a_reason() {
    let once = Repeat::Once {
        date: date(2026, 6, 1),
        time: time(13, 0, 0, 0),
    };
    let schedule = schedule(once, NOW);
    let plan = RoundPlanner::plan(at(NOW + HOUR + 1_000), at(NOW), &TimeZone::UTC, &[schedule]);
    assert_eq!(plan.runs.len(), 1);
    let changed = &plan.changed[0];
    assert!(!changed.switched_on);
    assert_eq!(changed.off_reason, None);
}

#[test]
fn a_clock_set_back_repeats_nothing() {
    let mut schedule = schedule(hourly(), NOW);
    let now = at(NOW + 5 * HOUR + 1_000);
    let first = RoundPlanner::plan(now, at(NOW), &TimeZone::UTC, &[schedule.clone()]);
    assert_eq!(first.runs.len(), 1);
    schedule = first.changed[0].clone();

    let back = at(NOW + 2 * HOUR);
    assert_eq!(
        RoundPlanner::plan(back, at(NOW), &TimeZone::UTC, &[schedule.clone()]),
        Plan::default()
    );
    // Back at the same moment as the last run: still nothing.
    assert_eq!(
        RoundPlanner::plan(now, at(NOW), &TimeZone::UTC, &[schedule]),
        Plan::default()
    );
}

#[test]
fn a_schedule_that_is_off_is_left_alone() {
    let mut schedule = schedule(hourly(), NOW);
    schedule.switched_on = false;
    assert_eq!(
        RoundPlanner::plan(at(NOW + 9 * HOUR), at(NOW), &TimeZone::UTC, &[schedule]),
        Plan::default()
    );
}

#[test]
fn a_hundred_year_jump_is_one_bounded_batch() {
    let schedule = schedule(hourly(), NOW);
    let now = at(NOW + 100 * 365 * 24 * HOUR + HOUR / 2);
    let started = std::time::Instant::now();
    let plan = RoundPlanner::plan(now, at(NOW), &TimeZone::UTC, &[schedule]);
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
    assert!(plan.missed[0].count > 800_000);
    assert_eq!(plan.runs.len(), 0);
}

#[test]
fn a_zone_change_gives_the_new_next_due() {
    let daily = Repeat::Daily {
        time: time(9, 0, 0, 0),
    };
    let in_utc = schedule(daily, NOW);
    assert_eq!(in_utc.next_due, Some(at(NOW + 21 * HOUR)));
    let berlin = TimeZone::get("Europe/Berlin").expect("zone");
    let plan = RoundPlanner::plan(at(NOW), at(NOW), &berlin, &[in_utc]);
    // 09:00 CEST on 2026-06-02 is 07:00 UTC.
    assert_eq!(plan.changed[0].next_due, Some(at(NOW + 19 * HOUR)));
    assert_eq!(plan.runs, []);
}

#[test]
fn a_due_time_within_the_grace_but_before_the_start_is_missed() {
    let schedule = schedule(hourly(), NOW);
    let now = at(NOW + HOUR + 1_000);
    let started = at(NOW + HOUR + 500);
    let plan = RoundPlanner::plan(now, started, &TimeZone::UTC, &[schedule]);
    assert_eq!(plan.runs, []);
    assert_eq!(plan.missed.len(), 1);
    assert_eq!(plan.missed[0].count, 1);
}

#[test]
fn a_once_schedule_with_no_due_time_left_is_switched_off() {
    let once = Repeat::Once {
        date: date(2026, 6, 1),
        time: time(13, 0, 0, 0),
    };
    let schedule = schedule(once, NOW + 2 * HOUR);
    assert!(schedule.switched_on && schedule.next_due.is_none());
    let plan = RoundPlanner::plan(at(NOW + 2 * HOUR), at(NOW), &TimeZone::UTC, &[schedule]);
    let changed = &plan.changed[0];
    assert!(!changed.switched_on);
    assert_eq!(changed.off_reason, Some(OffReason::TimeMissed));
    assert_eq!(plan.runs, []);
}
