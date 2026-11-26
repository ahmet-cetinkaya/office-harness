// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::ScheduleId;
use schedule_application::common::models::ScheduleRun;
use schedule_domain::{FieldError, Rule, Schedule};
use serde_json::{Map, Value, json};
use wire::Event;

use crate::contracts::repeat_json;

const SCHEDULE_CHANGED_EVENT: &str = "schedule.changed";
const SCHEDULE_REMOVED_EVENT: &str = "schedule.removed";

/// The wire form of one schedule. Times are unix milliseconds; the view
/// formats them in the zone that the list answer names.
#[must_use]
pub fn schedule_json(schedule: &Schedule) -> Value {
    let millis = |value: Option<kernel::Timestamp>| value.map(kernel::Timestamp::as_unix_millis);
    json!({
        "id": schedule.id.to_string(),
        "name": schedule.name,
        "text": schedule.text,
        "target_agent_id": schedule.target.to_string(),
        "repeat": repeat_json(&schedule.repeat),
        "on": schedule.switched_on,
        "next_due": millis(schedule.next_due),
        "last_handled_due": millis(schedule.last_handled_due),
        "last_missed": millis(schedule.last_missed),
        "off_reason": schedule.off_reason.map(schedule_domain::OffReason::as_str),
    })
}

/// The wire form of one history row.
#[must_use]
pub fn run_json(run: &ScheduleRun) -> Value {
    let outcome = run.outcome.as_ref();
    json!({
        "id": run.id.0,
        "schedule_id": run.schedule_id.to_string(),
        "due": run.due.as_unix_millis(),
        "started": run.started.as_unix_millis(),
        "via": run.via.as_str(),
        "result": outcome.map(|outcome| outcome.result.as_str()),
        "reason": outcome.and_then(|outcome| outcome.reason.as_deref()),
    })
}

fn rule_code(rule: Rule) -> &'static str {
    match rule {
        Rule::Blank => "blank",
        Rule::TooLong { .. } => "too_long",
        Rule::ControlCharacter => "control_character",
        Rule::NoDay => "no_day",
        Rule::HoursOutOfRange => "hours_out_of_range",
        Rule::NotInFuture => "not_in_future",
        Rule::BadTime => "bad_time",
        Rule::BadDate => "bad_date",
        Rule::NameTaken => "name_taken",
        Rule::TargetMissing => "target_missing",
    }
}

/// The `fields` array of an `invalid` answer: each failing field with its
/// rule as a code and as a sentence.
#[must_use]
pub fn field_errors_json(errors: &[FieldError]) -> Value {
    Value::Array(
        errors
            .iter()
            .map(|error| {
                json!({
                    "field": error.field,
                    "rule": rule_code(error.rule),
                    "message": error.rule.to_string(),
                })
            })
            .collect(),
    )
}

/// `schedule.changed`: the whole row after a save, a switch, or a round that
/// changed it.
#[must_use]
pub fn schedule_changed_event(schedule: &Schedule) -> Event {
    let mut fields = Map::new();
    fields.insert("schedule".to_owned(), schedule_json(schedule));
    Event::new(SCHEDULE_CHANGED_EVENT, fields)
}

/// `schedule.removed`: the id of a deleted schedule.
#[must_use]
pub fn schedule_removed_event(id: ScheduleId) -> Event {
    let mut fields = Map::new();
    fields.insert("id".to_owned(), Value::String(id.to_string()));
    Event::new(SCHEDULE_REMOVED_EVENT, fields)
}
