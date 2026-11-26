// SPDX-License-Identifier: AGPL-3.0-or-later

use jiff::civil::time;

use super::*;

fn schedule(repeat: Repeat) -> Schedule {
    Schedule {
        id: ScheduleId::from_u128(1),
        name: "Nightly".to_owned(),
        text: "go".to_owned(),
        target: AgentId::from_u128(2),
        repeat,
        switched_on: true,
        anchor: Timestamp::from_unix_millis(0),
        next_due: None,
        last_handled_due: None,
        last_missed: None,
        off_reason: None,
        notified_since_success: false,
    }
}

fn utc(millis: i64) -> Timestamp {
    Timestamp::from_unix_millis(millis)
}

const HOUR: i64 = 3_600_000;

#[test]
fn next_due_is_after_now() {
    let schedule = schedule(Repeat::EveryHours { hours: 2 });
    assert_eq!(
        schedule.next_due(utc(HOUR), &TimeZone::UTC),
        Some(utc(2 * HOUR))
    );
    assert_eq!(
        schedule.next_due(utc(2 * HOUR), &TimeZone::UTC),
        Some(utc(4 * HOUR))
    );
}

#[test]
fn a_clock_set_back_does_not_repeat_a_handled_due() {
    let mut schedule = schedule(Repeat::EveryHours { hours: 1 });
    schedule.last_handled_due = Some(utc(10 * HOUR));
    // The clock now reads 5 hours; the due times at 6 to 10 hours stay handled.
    assert_eq!(
        schedule.next_due(utc(5 * HOUR), &TimeZone::UTC),
        Some(utc(11 * HOUR))
    );
}

#[test]
fn a_zone_change_gives_a_new_next_due() {
    let schedule = schedule(Repeat::Daily {
        time: time(9, 0, 0, 0),
    });
    let new_york = TimeZone::get("America/New_York").expect("zone");
    let berlin = TimeZone::get("Europe/Berlin").expect("zone");
    let now = utc(1_780_315_200_000); // 2026-06-01 12:00 UTC
    let in_new_york = schedule.next_due(now, &new_york);
    let in_berlin = schedule.next_due(now, &berlin);
    assert_ne!(in_new_york, in_berlin);
    assert_eq!(in_new_york, Some(utc(1_780_318_800_000))); // 06-01 13:00 UTC
    assert_eq!(in_berlin, Some(utc(1_780_383_600_000))); // 06-02 07:00 UTC
}

#[test]
fn recompute_sets_next_due_only_while_on() {
    let mut schedule = schedule(Repeat::EveryHours { hours: 1 });
    schedule.recompute_next_due(utc(0), &TimeZone::UTC);
    assert_eq!(schedule.next_due, Some(utc(HOUR)));
    schedule.switched_on = false;
    schedule.recompute_next_due(utc(0), &TimeZone::UTC);
    assert_eq!(schedule.next_due, None);
}

#[test]
fn mark_handled_through_never_moves_back() {
    let mut schedule = schedule(Repeat::EveryHours { hours: 1 });
    schedule.mark_handled_through(utc(5 * HOUR));
    schedule.mark_handled_through(utc(3 * HOUR));
    assert_eq!(schedule.last_handled_due, Some(utc(5 * HOUR)));
}
