// SPDX-License-Identifier: AGPL-3.0-or-later

//! Schedule module domain: what a schedule is, when it is due (local time,
//! daylight saving, every-N-hours), and the rules each field must meet. No
//! I/O: the zone comes in through `ZoneSource` and the time as a parameter.

mod due;
mod repeat;
mod schedule;
mod schedule_cap;
mod validation;
mod zone_source;

pub use due::{
    DueWindow, MAX_STEPS, Missed, Split, TAIL_LEN, classify, due_times_in, next_due_after,
};
pub use repeat::{Repeat, parse_time, parse_weekday, time_text, weekday_code};
pub use schedule::{OffReason, Schedule};
pub use schedule_cap::{ScheduleCap, ScheduleCapError};
pub use validation::{
    FieldError, NAME_FIELD, NAME_MAX_CHARS, REPEAT_FIELD, Rule, TARGET_FIELD, TEXT_FIELD,
    TEXT_MAX_CHARS, validate_fields,
};
pub use zone_source::ZoneSource;
