// SPDX-License-Identifier: AGPL-3.0-or-later

use jiff::civil::{Date, Time, Weekday};

/// When a schedule is due. Once, daily, and weekly names a local civil time;
/// every-N-hours counts elapsed hours from the schedule's anchor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repeat {
    Once { date: Date, time: Time },
    Daily { time: Time },
    Weekly { days: Vec<Weekday>, time: Time },
    EveryHours { hours: u32 },
}

const WEEKDAY_CODES: [(Weekday, &str); 7] = [
    (Weekday::Monday, "mon"),
    (Weekday::Tuesday, "tue"),
    (Weekday::Wednesday, "wed"),
    (Weekday::Thursday, "thu"),
    (Weekday::Friday, "fri"),
    (Weekday::Saturday, "sat"),
    (Weekday::Sunday, "sun"),
];

/// The three-letter code of a weekday, as stored and as sent on the wire.
#[must_use]
pub fn weekday_code(day: Weekday) -> &'static str {
    WEEKDAY_CODES
        .iter()
        .find(|(candidate, _)| *candidate == day)
        .map_or("mon", |(_, code)| code)
}

#[must_use]
pub fn parse_weekday(code: &str) -> Option<Weekday> {
    WEEKDAY_CODES
        .iter()
        .find(|(_, candidate)| *candidate == code)
        .map(|(day, _)| *day)
}

/// A time as `HH:MM`; seconds are not part of a schedule time.
#[must_use]
pub fn time_text(time: Time) -> String {
    format!("{:02}:{:02}", time.hour(), time.minute())
}

/// Reads `HH:MM` with two digits each and a real hour and minute.
#[must_use]
pub fn parse_time(text: &str) -> Option<Time> {
    let (hour, minute) = text.split_once(':')?;
    if hour.len() != 2 || minute.len() != 2 {
        return None;
    }
    Time::new(hour.parse().ok()?, minute.parse().ok()?, 0, 0).ok()
}

#[cfg(test)]
#[path = "repeat.tests.rs"]
mod tests;
