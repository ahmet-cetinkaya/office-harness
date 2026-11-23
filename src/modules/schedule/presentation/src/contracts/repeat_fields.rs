// SPDX-License-Identifier: AGPL-3.0-or-later

use jiff::civil::Date;
use schedule_domain::{
    FieldError, REPEAT_FIELD, Repeat, Rule, parse_time, parse_weekday, time_text, weekday_code,
};
use serde_json::{Map, Value, json};

use super::Fields;

const REPEAT_KEY: &str = "repeat";

/// Why the `repeat` object of a request cannot be read: the request is
/// malformed, or a value breaks a rule of the `repeat` field.
#[derive(Debug, PartialEq, Eq)]
pub enum ReadError {
    Request(String),
    Rule(FieldError),
}

impl From<String> for ReadError {
    fn from(message: String) -> Self {
        Self::Request(format!("{REPEAT_KEY}.{message}"))
    }
}

fn rule(rule: Rule) -> ReadError {
    ReadError::Rule(FieldError {
        field: REPEAT_FIELD,
        rule,
    })
}

fn known_fields(kind: &str) -> Option<&'static [&'static str]> {
    match kind {
        "once" => Some(&["kind", "date", "time"]),
        "daily" => Some(&["kind", "time"]),
        "weekly" => Some(&["kind", "days", "time"]),
        "every_hours" => Some(&["kind", "hours"]),
        _ => None,
    }
}

/// Reads the `repeat` object of `schedule.save`:
/// `{"kind": "once", "date": "YYYY-MM-DD", "time": "HH:MM"}`,
/// `{"kind": "daily", "time"}`, `{"kind": "weekly", "days": ["mon", ...],
/// "time"}`, or `{"kind": "every_hours", "hours": N}`.
///
/// # Errors
/// `Request` for a missing field, a wrong type, or an unknown kind or key;
/// `Rule` for a time, a date, or an hours value that breaks its rule.
pub fn read_repeat(body: &Map<String, Value>) -> Result<Repeat, ReadError> {
    let fields = Fields(body);
    let kind = fields.required_text("kind")?;
    let allowed = known_fields(&kind)
        .ok_or_else(|| ReadError::Request(format!("{REPEAT_KEY}.kind {kind} is not a repeat")))?;
    if let Some(key) = body.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(ReadError::Request(format!(
            "{key} is not a field of {REPEAT_KEY} {kind}"
        )));
    }
    match kind.as_str() {
        "once" => Ok(Repeat::Once {
            date: fields
                .required_text("date")?
                .parse::<Date>()
                .map_err(|_| rule(Rule::BadDate))?,
            time: read_time(&fields)?,
        }),
        "daily" => Ok(Repeat::Daily {
            time: read_time(&fields)?,
        }),
        "weekly" => Ok(Repeat::Weekly {
            days: read_days(body)?,
            time: read_time(&fields)?,
        }),
        _ => read_hours(body),
    }
}

fn read_time(fields: &Fields<'_>) -> Result<jiff::civil::Time, ReadError> {
    parse_time(&fields.required_text("time")?).ok_or_else(|| rule(Rule::BadTime))
}

fn read_days(body: &Map<String, Value>) -> Result<Vec<jiff::civil::Weekday>, ReadError> {
    let Some(Value::Array(days)) = body.get("days") else {
        return Err(ReadError::Request(format!(
            "{REPEAT_KEY}.days must be a list of weekday codes"
        )));
    };
    days.iter()
        .map(|day| {
            day.as_str().and_then(parse_weekday).ok_or_else(|| {
                ReadError::Request(format!(
                    "{REPEAT_KEY}.days must hold only mon, tue, wed, thu, fri, sat, or sun"
                ))
            })
        })
        .collect()
}

/// A number that is not a whole one, or that does not fit, is out of the
/// range 1 to 168 by the same rule as 0 or 169.
fn read_hours(body: &Map<String, Value>) -> Result<Repeat, ReadError> {
    let Some(Value::Number(number)) = body.get("hours") else {
        return Err(ReadError::Request(format!(
            "{REPEAT_KEY}.hours must be a number"
        )));
    };
    let hours = number
        .as_i64()
        .map(|whole| u32::try_from(whole).unwrap_or(u32::MAX))
        .ok_or_else(|| rule(Rule::HoursOutOfRange))?;
    Ok(Repeat::EveryHours { hours })
}

/// The wire form of a repeat, as `read_repeat` reads it.
#[must_use]
pub fn repeat_json(repeat: &Repeat) -> Value {
    match repeat {
        Repeat::Once { date, time } => {
            json!({"kind": "once", "date": date.to_string(), "time": time_text(*time)})
        }
        Repeat::Daily { time } => json!({"kind": "daily", "time": time_text(*time)}),
        Repeat::Weekly { days, time } => json!({
            "kind": "weekly",
            "days": days.iter().map(|day| weekday_code(*day)).collect::<Vec<_>>(),
            "time": time_text(*time),
        }),
        Repeat::EveryHours { hours } => json!({"kind": "every_hours", "hours": hours}),
    }
}
