// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::{Duration, Instant};

use jiff::civil::{Weekday, date, time};

use super::*;

const HOUR: i64 = 3_600_000;
const MINUTE: i64 = 60_000;
const GRACE: i64 = 120_000;

fn zone(name: &str) -> TimeZone {
    TimeZone::get(name).expect("zone")
}

/// The unix millis of a local time in a zone, resolved as the rules do.
fn local(name: &str, year: i16, month: i8, day: i8, hour: i8, minute: i8) -> Timestamp {
    local_due(
        &zone(name),
        date(year, month, day),
        time(hour, minute, 0, 0),
    )
    .expect("in range")
}

fn utc(year: i16, month: i8, day: i8, hour: i8, minute: i8) -> Timestamp {
    local("UTC", year, month, day, hour, minute)
}

fn millis(value: i64) -> Timestamp {
    Timestamp::from_unix_millis(value)
}

fn daily(hour: i8, minute: i8) -> Repeat {
    Repeat::Daily {
        time: time(hour, minute, 0, 0),
    }
}

fn next(repeat: &Repeat, anchor: Timestamp, name: &str, after: Timestamp) -> Option<Timestamp> {
    next_due_after(repeat, anchor, &zone(name), after)
}

fn window(
    repeat: &Repeat,
    anchor: Timestamp,
    name: &str,
    after: Timestamp,
    up_to: Timestamp,
) -> DueWindow {
    due_times_in(repeat, anchor, &zone(name), after, up_to, MAX_STEPS)
}

#[test]
fn once_is_due_once_and_only_in_the_future() {
    let repeat = Repeat::Once {
        date: date(2026, 6, 10),
        time: time(9, 30, 0, 0),
    };
    let due = utc(2026, 6, 10, 9, 30);
    let anchor = millis(0);
    assert_eq!(
        next(&repeat, anchor, "UTC", utc(2026, 6, 1, 0, 0)),
        Some(due)
    );
    assert_eq!(next(&repeat, anchor, "UTC", due), None);
    assert_eq!(next(&repeat, anchor, "UTC", utc(2026, 6, 11, 0, 0)), None);
}

#[test]
fn daily_is_due_each_local_day() {
    let repeat = daily(8, 0);
    let anchor = millis(0);
    assert_eq!(
        next(&repeat, anchor, "UTC", utc(2026, 6, 1, 7, 59)),
        Some(utc(2026, 6, 1, 8, 0))
    );
    assert_eq!(
        next(&repeat, anchor, "UTC", utc(2026, 6, 1, 8, 0)),
        Some(utc(2026, 6, 2, 8, 0))
    );
    assert_eq!(
        next(&repeat, anchor, "Europe/Berlin", utc(2026, 6, 1, 6, 0)),
        Some(local("Europe/Berlin", 2026, 6, 2, 8, 0))
    );
}

#[test]
fn weekly_is_due_on_the_named_days() {
    let repeat = Repeat::Weekly {
        days: vec![Weekday::Monday, Weekday::Wednesday],
        time: time(9, 0, 0, 0),
    };
    let anchor = millis(0);
    // 2026-06-03 is a Wednesday.
    assert_eq!(
        next(&repeat, anchor, "UTC", utc(2026, 6, 3, 10, 0)),
        Some(utc(2026, 6, 8, 9, 0))
    );
    assert_eq!(
        next(&repeat, anchor, "UTC", utc(2026, 6, 3, 8, 0)),
        Some(utc(2026, 6, 3, 9, 0))
    );
    let all = window(
        &repeat,
        anchor,
        "UTC",
        utc(2026, 6, 1, 0, 0),
        utc(2026, 6, 15, 23, 0),
    );
    assert_eq!(all.count, 5); // Mon 1, Wed 3, Mon 8, Wed 10, Mon 15
}

#[test]
fn every_n_hours_counts_whole_multiples_after_the_anchor() {
    let anchor = utc(2026, 6, 1, 10, 7);
    let repeat = Repeat::EveryHours { hours: 3 };
    assert_eq!(
        next(&repeat, anchor, "UTC", utc(2026, 5, 1, 0, 0)),
        Some(utc(2026, 6, 1, 13, 7))
    );
    assert_eq!(
        next(&repeat, anchor, "UTC", anchor),
        Some(utc(2026, 6, 1, 13, 7))
    );
    assert_eq!(
        next(&repeat, anchor, "UTC", utc(2026, 6, 1, 13, 7)),
        Some(utc(2026, 6, 1, 16, 7))
    );
    assert_eq!(
        next(&repeat, anchor, "UTC", utc(2026, 6, 1, 13, 6)),
        Some(utc(2026, 6, 1, 13, 7))
    );
}

#[test]
fn spring_forward_runs_a_skipped_time_once_after_the_gap() {
    let repeat = daily(2, 30);
    let ny = "America/New_York";
    let after = local(ny, 2026, 3, 7, 12, 0);
    let spring = next(&repeat, millis(0), ny, after).expect("due");
    // 02:30 does not exist on 2026-03-08: it runs at 03:00 EDT, 07:00 UTC.
    assert_eq!(spring, utc(2026, 3, 8, 7, 0));
    let days = window(&repeat, millis(0), ny, after, local(ny, 2026, 3, 9, 12, 0));
    assert_eq!(days.count, 2);
    assert_eq!(
        days.tail,
        [utc(2026, 3, 8, 7, 0), local(ny, 2026, 3, 9, 2, 30)]
    );
}

#[test]
fn fall_back_runs_a_repeated_time_once_at_its_first_occurrence() {
    let repeat = daily(1, 30);
    let ny = "America/New_York";
    let days = window(
        &repeat,
        millis(0),
        ny,
        local(ny, 2026, 10, 31, 12, 0),
        local(ny, 2026, 11, 2, 12, 0),
    );
    assert_eq!(days.count, 2);
    // 01:30 occurs twice on 2026-11-01; the first is EDT, 05:30 UTC.
    assert_eq!(days.tail[0], utc(2026, 11, 1, 5, 30));
    assert_eq!(
        next(&repeat, millis(0), ny, utc(2026, 11, 1, 5, 30)),
        Some(local(ny, 2026, 11, 2, 1, 30))
    );
}

#[test]
fn berlin_changes_follow_the_same_rules() {
    let repeat = daily(2, 30);
    let berlin = "Europe/Berlin";
    // Spring: 02:30 does not exist on 2026-03-29; 03:00 CEST is 01:00 UTC.
    assert_eq!(
        next(&repeat, millis(0), berlin, utc(2026, 3, 28, 12, 0)),
        Some(utc(2026, 3, 29, 1, 0))
    );
    // Autumn: 02:30 occurs twice on 2026-10-25; the first is CEST, 00:30 UTC.
    assert_eq!(
        next(&repeat, millis(0), berlin, utc(2026, 10, 24, 12, 0)),
        Some(utc(2026, 10, 25, 0, 30))
    );
    let both = window(
        &repeat,
        millis(0),
        berlin,
        utc(2026, 10, 24, 12, 0),
        utc(2026, 10, 25, 23, 0),
    );
    assert_eq!(both.count, 1);
}

#[test]
fn every_two_hours_keeps_two_elapsed_hours_across_both_changes() {
    let repeat = Repeat::EveryHours { hours: 2 };
    let anchor = utc(2026, 3, 1, 0, 0);
    let ny = zone("America/New_York");
    let mut due = anchor;
    let end = utc(2026, 11, 3, 0, 0);
    let mut count = 0;
    while let Some(following) = next_due_after(&repeat, anchor, &ny, due).filter(|due| *due <= end)
    {
        assert_eq!(following.as_unix_millis() - due.as_unix_millis(), 2 * HOUR);
        due = following;
        count += 1;
    }
    let counted = window(&repeat, anchor, "America/New_York", anchor, end);
    assert_eq!(counted.count, count);
}

#[test]
fn a_window_counts_each_due_time_with_its_first_and_last() {
    let anchor = utc(2026, 6, 1, 0, 0);
    let repeat = Repeat::EveryHours { hours: 1 };
    let counted = window(
        &repeat,
        anchor,
        "UTC",
        utc(2026, 6, 1, 0, 0),
        utc(2026, 6, 1, 5, 0),
    );
    assert_eq!(counted.count, 5);
    assert_eq!(counted.first, Some(utc(2026, 6, 1, 1, 0)));
    assert_eq!(
        counted.tail,
        [
            utc(2026, 6, 1, 3, 0),
            utc(2026, 6, 1, 4, 0),
            utc(2026, 6, 1, 5, 0)
        ]
    );
    assert!(!counted.truncated);
    assert_eq!(
        window(
            &repeat,
            anchor,
            "UTC",
            utc(2026, 6, 1, 5, 0),
            utc(2026, 6, 1, 5, 0)
        ),
        DueWindow::default()
    );
    assert_eq!(
        window(
            &repeat,
            anchor,
            "UTC",
            utc(2026, 6, 1, 5, 0),
            utc(2026, 6, 1, 5, 59)
        )
        .count,
        0
    );
}

#[test]
fn a_hundred_year_jump_is_bounded() {
    let started = Instant::now();
    let from = utc(2026, 6, 1, 0, 0);
    let to = utc(2126, 6, 1, 0, 0);
    let hourly = window(&Repeat::EveryHours { hours: 1 }, from, "UTC", from, to);
    assert_eq!(hourly.count, 876_576); // 36 524 days of 24 hours
    assert!(!hourly.truncated);
    assert_eq!(hourly.last(), Some(to));

    let every_day = window(&daily(8, 0), from, "America/New_York", from, to);
    assert!(!every_day.truncated);
    assert!(every_day.count > 36_000);

    // A thousand years of days passes the step limit.
    let far = utc(3026, 6, 1, 0, 0);
    let truncated = window(&daily(8, 0), from, "UTC", from, far);
    assert!(truncated.truncated);
    assert_eq!(truncated.last(), Some(utc(3026, 5, 31, 8, 0)));
    assert_eq!(truncated.first, Some(utc(2026, 6, 1, 8, 0)));
    assert!(started.elapsed() < Duration::from_secs(20));
}

#[test]
fn a_small_step_limit_truncates_and_still_finds_the_last_due_times() {
    let from = utc(2026, 1, 1, 0, 0);
    let to = utc(2026, 12, 31, 12, 0);
    let limited = due_times_in(&daily(8, 0), from, &zone("UTC"), from, to, 10);
    assert!(limited.truncated);
    assert_eq!(limited.first, Some(utc(2026, 1, 1, 8, 0)));
    assert_eq!(limited.last(), Some(utc(2026, 12, 31, 8, 0)));
    assert_eq!(limited.tail.len(), TAIL_LEN);
}

#[test]
fn classify_runs_what_is_within_the_grace_and_misses_the_rest() {
    let anchor = utc(2026, 6, 1, 0, 0);
    let repeat = Repeat::EveryHours { hours: 1 };
    let now = utc(2026, 6, 1, 5, 1);
    let counted = window(&repeat, anchor, "UTC", anchor, now);
    let split = classify(&counted, now, GRACE);
    assert_eq!(split.runs, [utc(2026, 6, 1, 5, 0)]);
    assert_eq!(
        split.missed,
        Some(Missed {
            count: 4,
            first: utc(2026, 6, 1, 1, 0),
            last: utc(2026, 6, 1, 4, 0)
        })
    );
}

#[test]
fn classify_has_no_missed_batch_when_every_due_time_is_fresh() {
    let anchor = utc(2026, 6, 1, 0, 0);
    let repeat = Repeat::EveryHours { hours: 1 };
    let now = utc(2026, 6, 1, 1, 0).as_unix_millis() + GRACE;
    let split = classify(
        &window(&repeat, anchor, "UTC", anchor, millis(now)),
        millis(now),
        GRACE,
    );
    assert_eq!(split.runs, [utc(2026, 6, 1, 1, 0)]);
    assert_eq!(split.missed, None);
}

#[test]
fn classify_misses_all_when_none_is_within_the_grace() {
    let anchor = utc(2026, 6, 1, 0, 0);
    let repeat = Repeat::EveryHours { hours: 1 };
    let now = millis(utc(2026, 6, 1, 3, 0).as_unix_millis() + GRACE + MINUTE);
    let split = classify(&window(&repeat, anchor, "UTC", anchor, now), now, GRACE);
    assert_eq!(split.runs, []);
    assert_eq!(
        split.missed,
        Some(Missed {
            count: 3,
            first: utc(2026, 6, 1, 1, 0),
            last: utc(2026, 6, 1, 3, 0)
        })
    );
}

#[test]
fn classify_an_empty_window_is_empty() {
    let split = classify(&DueWindow::default(), millis(0), GRACE);
    assert_eq!(split.runs, []);
    assert_eq!(split.missed, None);
}
