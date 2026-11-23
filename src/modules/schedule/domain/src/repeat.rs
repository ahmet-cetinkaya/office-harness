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
