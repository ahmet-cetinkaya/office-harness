// SPDX-License-Identifier: AGPL-3.0-or-later

//! Due times. Once, daily, and weekly repeats name a civil time in the
//! machine's zone: a time that daylight saving skips becomes the first minute
//! after the gap, and a time that occurs twice takes its first occurrence.
//! (`jiff`'s `Compatible` shifts a skipped time by the length of the gap, so
//! 02:30 would become 03:30; the rule wants 03:00, the end of the gap.) An
//! every-N-hours repeat is
//! the anchor plus whole multiples of N hours of elapsed time, in no zone.

use jiff::civil::{Date, Time};
use jiff::tz::{AmbiguousOffset, TimeZone};
use kernel::Timestamp;

use crate::Repeat;

/// Steps (days) a due-time scan takes before it reports `truncated`.
pub const MAX_STEPS: usize = 200_000;
/// Due times kept at the end of a window; the spacing of 1 hour or more means
/// that this covers every due time within any grace shorter than that.
pub const TAIL_LEN: usize = 3;

const MILLIS_PER_HOUR: i64 = 3_600_000;
/// A day search for the next due time: a weekly repeat is at most 7 days away,
/// and a gap or a fold moves a time by under a day.
const NEXT_DUE_SEARCH_DAYS: usize = 9;
/// Days scanned at the end of a truncated window to find its last due times.
const TAIL_SCAN_DAYS: usize = 9;

/// The due times in one window, bounded in size however long the window is.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DueWindow {
    /// How many due times the window holds; a lower bound when `truncated`.
    pub count: u64,
    pub first: Option<Timestamp>,
    /// The last due times of the window, oldest first, at most `TAIL_LEN`.
    pub tail: Vec<Timestamp>,
    /// The scan stopped at its step limit: `count` is a lower bound.
    pub truncated: bool,
}

impl DueWindow {
    #[must_use]
    pub fn last(&self) -> Option<Timestamp> {
        self.tail.last().copied()
    }

    fn push(&mut self, due: Timestamp) {
        self.count += 1;
        self.first.get_or_insert(due);
        if self.tail.len() == TAIL_LEN {
            self.tail.remove(0);
        }
        self.tail.push(due);
    }
}

/// A due time as local civil time resolved in the zone; `None` when the time
/// is outside the range that the zone arithmetic supports.
pub(crate) fn local_due(zone: &TimeZone, date: Date, time: Time) -> Option<Timestamp> {
    let ambiguous = zone.to_ambiguous_timestamp(date.to_datetime(time));
    let resolved = match ambiguous.offset() {
        AmbiguousOffset::Gap { .. } => end_of_gap(zone, ambiguous.earlier().ok()?)?,
        _ => ambiguous.compatible().ok()?,
    };
    Some(Timestamp::from_unix_millis(resolved.as_millisecond()))
}

/// The transition that ends the gap, from an instant just before it.
fn end_of_gap(zone: &TimeZone, before_gap: jiff::Timestamp) -> Option<jiff::Timestamp> {
    zone.following(before_gap)
        .next()
        .map(|transition| transition.timestamp())
}

fn date_of(zone: &TimeZone, at: Timestamp) -> Option<Date> {
    let instant = jiff::Timestamp::from_millisecond(at.as_unix_millis()).ok()?;
    Some(zone.to_datetime(instant).date())
}

/// The due time on one local date, for the repeats that name a civil time.
fn due_on(repeat: &Repeat, zone: &TimeZone, date: Date) -> Option<Timestamp> {
    let time = match repeat {
        Repeat::Once { date: once, time } if *once == date => *time,
        Repeat::Daily { time } => *time,
        Repeat::Weekly { days, time } if days.contains(&date.weekday()) => *time,
        _ => return None,
    };
    local_due(zone, date, time)
}

fn step_millis(hours: u32) -> Option<i64> {
    (hours > 0).then(|| i64::from(hours) * MILLIS_PER_HOUR)
}

/// How many due times of an every-N-hours repeat are at or before `at`.
fn elapsed_steps(anchor: Timestamp, step: i64, at: Timestamp) -> i64 {
    let elapsed = at.as_unix_millis().saturating_sub(anchor.as_unix_millis());
    if elapsed < 0 { 0 } else { elapsed / step }
}

fn nth_due(anchor: Timestamp, step: i64, step_count: i64) -> Option<Timestamp> {
    step.checked_mul(step_count)
        .and_then(|offset| anchor.as_unix_millis().checked_add(offset))
        .map(Timestamp::from_unix_millis)
}

/// The first due time later than `after`; `None` for a once repeat that is
/// past, or a repeat with no due time left in range. A civil repeat does not
/// use the anchor.
#[must_use]
pub fn next_due_after(
    repeat: &Repeat,
    anchor: Timestamp,
    zone: &TimeZone,
    after: Timestamp,
) -> Option<Timestamp> {
    match repeat {
        Repeat::Once { date, time } => local_due(zone, *date, *time).filter(|due| *due > after),
        Repeat::EveryHours { hours } => {
            let step = step_millis(*hours)?;
            nth_due(
                anchor,
                step,
                elapsed_steps(anchor, step, after).checked_add(1)?,
            )
        }
        Repeat::Daily { .. } | Repeat::Weekly { .. } => {
            let mut date = date_of(zone, after)?.yesterday().ok()?;
            for _ in 0..NEXT_DUE_SEARCH_DAYS {
                if let Some(due) = due_on(repeat, zone, date).filter(|due| *due > after) {
                    return Some(due);
                }
                date = date.tomorrow().ok()?;
            }
            None
        }
    }
}

/// Every due time with `after_exclusive < due <= up_to_inclusive`, summed up
/// in a bounded `DueWindow`. An every-N-hours window is counted by arithmetic
/// and is never truncated. A civil window steps by day, at most `limit` days
/// (callers pass `MAX_STEPS`); past that it reports `truncated`, with a count
/// that is a lower bound, and still finds the last due times of the window.
#[must_use]
pub fn due_times_in(
    repeat: &Repeat,
    anchor: Timestamp,
    zone: &TimeZone,
    after_exclusive: Timestamp,
    up_to_inclusive: Timestamp,
    limit: usize,
) -> DueWindow {
    if up_to_inclusive <= after_exclusive {
        return DueWindow::default();
    }
    match repeat {
        Repeat::EveryHours { hours } => step_millis(*hours)
            .map_or_else(DueWindow::default, |step| {
                hours_window(anchor, step, after_exclusive, up_to_inclusive)
            }),
        Repeat::Once { .. } | Repeat::Daily { .. } | Repeat::Weekly { .. } => {
            civil_window(repeat, zone, after_exclusive, up_to_inclusive, limit)
        }
    }
}

fn hours_window(anchor: Timestamp, step: i64, after: Timestamp, up_to: Timestamp) -> DueWindow {
    let before = elapsed_steps(anchor, step, after);
    let through = elapsed_steps(anchor, step, up_to);
    let mut window = DueWindow {
        count: u64::try_from(through - before).unwrap_or(0),
        first: nth_due(anchor, step, before + 1).filter(|_| through > before),
        ..DueWindow::default()
    };
    window.tail = (before + 1..=through)
        .rev()
        .take(TAIL_LEN)
        .filter_map(|step_count| nth_due(anchor, step, step_count))
        .collect();
    window.tail.reverse();
    window
}

fn civil_window(
    repeat: &Repeat,
    zone: &TimeZone,
    after: Timestamp,
    up_to: Timestamp,
    limit: usize,
) -> DueWindow {
    let mut window = DueWindow::default();
    let (Some(start), Some(end)) = (
        date_of(zone, after).and_then(|date| date.yesterday().ok()),
        date_of(zone, up_to).and_then(|date| date.tomorrow().ok()),
    ) else {
        return window;
    };
    let collect = |date: Date, window: &mut DueWindow| {
        if let Some(due) = due_on(repeat, zone, date).filter(|due| *due > after && *due <= up_to) {
            window.push(due);
        }
    };

    let mut date = start;
    let mut steps = 0;
    while date <= end && steps < limit {
        collect(date, &mut window);
        steps += 1;
        let Ok(next) = date.tomorrow() else {
            return window;
        };
        date = next;
    }
    if date > end {
        return window;
    }

    window.truncated = true;
    let mut tail_date = tail_scan_start(end).max(date);
    while tail_date <= end {
        collect(tail_date, &mut window);
        let Ok(next) = tail_date.tomorrow() else {
            break;
        };
        tail_date = next;
    }
    window
}

fn tail_scan_start(end: Date) -> Date {
    let mut date = end;
    for _ in 0..TAIL_SCAN_DAYS {
        match date.yesterday() {
            Ok(earlier) => date = earlier,
            Err(_) => break,
        }
    }
    date
}

/// A missed batch: how many due times, and the first and last of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Missed {
    pub count: u64,
    pub first: Timestamp,
    pub last: Timestamp,
}

/// A window split into the due times that run now and the ones that missed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Split {
    pub runs: Vec<Timestamp>,
    pub missed: Option<Missed>,
}

/// A due time `d` runs when it is not before `started` (the moment the office
/// began to run) and `now - d <= grace_millis`; every other one joins one
/// missed batch. `grace_millis` must be under the 1 hour spacing of due
/// times, so the tail holds every run.
#[must_use]
pub fn classify(
    window: &DueWindow,
    now: Timestamp,
    started: Timestamp,
    grace_millis: i64,
) -> Split {
    let is_run = |due: &Timestamp| {
        *due >= started && now.as_unix_millis() - due.as_unix_millis() <= grace_millis
    };
    let runs: Vec<Timestamp> = window.tail.iter().copied().filter(is_run).collect();
    let missed_count = window.count.saturating_sub(runs.len() as u64);
    let last_missed = window.tail.iter().rev().find(|due| !is_run(due)).copied();
    let missed = match (window.first, last_missed) {
        (Some(first), Some(last)) if missed_count > 0 => Some(Missed {
            count: missed_count,
            first,
            last,
        }),
        _ => None,
    };
    Split { runs, missed }
}

#[cfg(test)]
#[path = "due.tests.rs"]
mod tests;
