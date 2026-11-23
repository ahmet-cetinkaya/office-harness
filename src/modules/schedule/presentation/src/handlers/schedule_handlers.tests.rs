// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Mutex, PoisonError};

use jiff::tz::TimeZone;
use kernel::Timestamp;
use schedule_application::common::errors::ScheduleApplicationError;
use schedule_application::common::interfaces::{
    ScheduleCapSource, ScheduleClock, ScheduleStore, TargetCheck,
};
use schedule_application::common::models::{NewRun, RunId, RunOutcome, ScheduleRun};
use schedule_domain::{Schedule, ScheduleCap, ZoneSource};
use serde_json::{Map, json};
use wire::ErrorCode;

use super::*;
use crate::contracts::refuse_unknown_fields;

/// 2026-06-01 12:00 UTC.
const NOW: i64 = 1_780_315_200_000;

#[derive(Default)]
struct MemorySchedules(Mutex<Vec<Schedule>>);

impl MemorySchedules {
    fn rows(&self) -> std::sync::MutexGuard<'_, Vec<Schedule>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl ScheduleStore for MemorySchedules {
    fn list(&self) -> Result<Vec<Schedule>, ScheduleApplicationError> {
        Ok(self.rows().clone())
    }

    fn get(&self, id: ScheduleId) -> Result<Option<Schedule>, ScheduleApplicationError> {
        Ok(self.rows().iter().find(|row| row.id == id).cloned())
    }

    fn save(&self, schedule: &Schedule, cap: ScheduleCap) -> Result<(), ScheduleApplicationError> {
        let mut rows = self.rows();
        let key = schedule.name.to_lowercase();
        if rows
            .iter()
            .any(|row| row.id != schedule.id && row.name.to_lowercase() == key)
        {
            return Err(ScheduleApplicationError::NameTaken {
                name: schedule.name.clone(),
            });
        }
        if let Some(row) = rows.iter_mut().find(|row| row.id == schedule.id) {
            *row = schedule.clone();
            return Ok(());
        }
        if rows.len() >= cap.get() as usize {
            return Err(ScheduleApplicationError::CapReached { cap: cap.get() });
        }
        rows.push(schedule.clone());
        Ok(())
    }

    fn delete(&self, id: ScheduleId) -> Result<bool, ScheduleApplicationError> {
        let mut rows = self.rows();
        let before = rows.len();
        rows.retain(|row| row.id != id);
        Ok(rows.len() != before)
    }

    fn record_run(&self, _run: &NewRun) -> Result<RunId, ScheduleApplicationError> {
        unreachable!("these tests never record a run")
    }

    fn get_run(&self, _id: RunId) -> Result<Option<ScheduleRun>, ScheduleApplicationError> {
        unreachable!("these tests never read a run")
    }

    fn settle_run(
        &self,
        _id: RunId,
        _outcome: &RunOutcome,
    ) -> Result<bool, ScheduleApplicationError> {
        unreachable!("these tests never settle a run")
    }

    fn history(&self, _id: ScheduleId) -> Result<Vec<ScheduleRun>, ScheduleApplicationError> {
        Ok(Vec::new())
    }

    fn fail_open_runs(&self, _reason: &str) -> Result<usize, ScheduleApplicationError> {
        unreachable!("these tests never start a server")
    }
}

struct FixedClock;

impl ScheduleClock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_millis(NOW)
    }
}

struct Cap(u32);

impl ScheduleCapSource for Cap {
    fn cap(&self) -> ScheduleCap {
        ScheduleCap::new(i64::from(self.0)).unwrap_or_default()
    }
}

struct OnlyTarget;

impl TargetCheck for OnlyTarget {
    fn exists(&self, agent_id: AgentId) -> bool {
        agent_id == AgentId::from_u128(42)
    }
}

struct Berlin;

impl ZoneSource for Berlin {
    fn current(&self) -> TimeZone {
        TimeZone::get("Europe/Berlin").unwrap_or(TimeZone::UTC)
    }
}

#[derive(Default)]
struct RecordingRunNow(Mutex<Vec<ScheduleId>>);

impl RunNow for RecordingRunNow {
    fn run_now(&self, schedule: &Schedule) -> Result<(), String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(schedule.id);
        Ok(())
    }
}

struct RefusingRunNow;

impl RunNow for RefusingRunNow {
    fn run_now(&self, _schedule: &Schedule) -> Result<(), String> {
        Err("the office is on hold".to_owned())
    }
}

fn handlers_with(cap: u32, run_now: Arc<dyn RunNow>) -> ScheduleHandlers {
    ScheduleHandlers::new(
        ScheduleService::new(
            Arc::new(MemorySchedules::default()),
            Arc::new(FixedClock),
            Arc::new(Cap(cap)),
            Arc::new(OnlyTarget),
            Arc::new(Berlin),
        ),
        run_now,
    )
}

fn handlers() -> ScheduleHandlers {
    handlers_with(50, Arc::new(RecordingRunNow::default()))
}

fn request(kind: &str, fields: &Value) -> Request {
    let Value::Object(fields) = fields else {
        panic!("an object");
    };
    Request {
        id: 7,
        kind: kind.to_owned(),
        fields: Map::from_iter(fields.clone()),
    }
}

fn save_fields(name: &str) -> Value {
    json!({
        "name": name,
        "text": "ship the report",
        "target_agent_id": AgentId::from_u128(42).to_string(),
        "repeat": {"kind": "daily", "time": "08:00"},
        "on": true,
    })
}

fn failure_of(handled: &Handled) -> (ErrorCode, &str) {
    match &handled.response {
        Response::Failure { code, message, .. } => (*code, message.as_str()),
        other @ Response::Success { .. } => panic!("expected a failure, got {other:?}"),
    }
}

fn result_of(handled: &Handled) -> &Value {
    match &handled.response {
        Response::Success { result, .. } => result,
        other @ Response::Failure { .. } => panic!("expected a success, got {other:?}"),
    }
}

fn saved_id(handlers: &ScheduleHandlers, name: &str) -> String {
    let handled = handlers.handle(&request(SAVE, &save_fields(name)));
    result_of(&handled)["schedule"]["id"]
        .as_str()
        .expect("id")
        .to_owned()
}

#[test]
fn a_field_the_request_does_not_take_is_named() {
    let misspelt = request("schedule.delete", &json!({"scheduleid": "x"}));
    assert_eq!(
        refuse_unknown_fields(&misspelt),
        Err("scheduleid is not a field of schedule.delete".to_owned())
    );
}

#[test]
fn saving_answers_with_the_schedule_and_announces_it() {
    let handled = handlers().handle(&request(SAVE, &save_fields("Nightly")));

    let schedule = &result_of(&handled)["schedule"];
    assert_eq!(schedule["name"], "Nightly");
    assert_eq!(schedule["on"], true);
    assert_eq!(
        schedule["repeat"],
        json!({"kind": "daily", "time": "08:00"})
    );
    // 08:00 in Berlin (CEST) on 2026-06-02 is 06:00 UTC.
    assert_eq!(schedule["next_due"], NOW + 18 * 3_600_000);
    assert_eq!(schedule["off_reason"], Value::Null);
    assert_eq!(handled.events.len(), 1);
    assert!(
        handled.events[0]
            .encode()
            .contains("\"type\":\"schedule.changed\"")
    );
}

#[test]
fn the_list_carries_the_zone_for_the_view() {
    let handlers = handlers();
    saved_id(&handlers, "Nightly");
    let handled = handlers.handle(&request(LIST, &json!({})));
    assert_eq!(result_of(&handled)["zone"], "Europe/Berlin");
    assert_eq!(
        result_of(&handled)["schedules"]
            .as_array()
            .expect("list")
            .len(),
        1
    );
}

#[test]
fn a_failed_rule_is_invalid_with_a_fields_array_of_field_and_rule() {
    let mut fields = save_fields("");
    fields["text"] = json!("");
    fields["repeat"] = json!({"kind": "weekly", "days": [], "time": "08:00"});

    let handled = handlers().handle(&request(SAVE, &fields));

    assert_eq!(failure_of(&handled).0, ErrorCode::Invalid);
    let array = handled.error_fields.expect("fields");
    let pairs: Vec<(&str, &str)> = array
        .as_array()
        .expect("array")
        .iter()
        .map(|entry| {
            (
                entry["field"].as_str().expect("field"),
                entry["rule"].as_str().expect("rule"),
            )
        })
        .collect();
    assert_eq!(
        pairs,
        [("name", "blank"), ("text", "blank"), ("repeat", "no_day")]
    );
    assert_eq!(
        array[0],
        json!({"field": "name", "rule": "blank", "message": "must not be blank"})
    );
}

#[test]
fn hours_that_are_not_whole_or_out_of_range_are_a_repeat_rule() {
    for hours in [
        json!(0),
        json!(169),
        json!(2.5),
        json!(-3),
        json!(5_000_000_000_i64),
    ] {
        let mut fields = save_fields("Hourly");
        fields["repeat"] = json!({"kind": "every_hours", "hours": hours});
        let handled = handlers().handle(&request(SAVE, &fields));
        let array = handled.error_fields.expect("fields");
        assert_eq!(array[0]["field"], "repeat", "{hours}");
        assert_eq!(array[0]["rule"], "hours_out_of_range", "{hours}");
    }
}

#[test]
fn a_bad_time_date_or_target_is_named() {
    let mut fields = save_fields("Once");
    fields["repeat"] = json!({"kind": "once", "date": "2026-02-30", "time": "25:00"});
    fields["target_agent_id"] = json!("not-an-id");
    let handled = handlers().handle(&request(SAVE, &fields));
    let array = handled.error_fields.expect("fields");
    assert_eq!(
        array[0],
        json!({"field": "target", "rule": "target_missing", "message": "must name an agent that exists"})
    );
    assert_eq!(array[1]["rule"], "bad_date");

    let mut fields = save_fields("Daily");
    fields["repeat"] = json!({"kind": "daily", "time": "8:00"});
    let handled = handlers().handle(&request(SAVE, &fields));
    assert_eq!(handled.error_fields.expect("fields")[0]["rule"], "bad_time");
}

#[test]
fn a_malformed_request_is_invalid_without_a_fields_array() {
    for fields in [
        json!({"name": "x"}),
        json!({"name": "x", "text": "t", "target_agent_id": "a", "repeat": {"kind": "monthly"}, "on": true}),
        json!({"name": "x", "text": "t", "target_agent_id": "a", "repeat": {"kind": "daily", "time": "08:00", "day": 1}, "on": true}),
        json!({"name": "x", "text": "t", "target_agent_id": "a", "repeat": {"kind": "weekly", "days": ["funday"], "time": "08:00"}, "on": true}),
        json!({"name": "x", "text": "t", "target_agent_id": "a", "repeat": {"kind": "daily", "time": "08:00"}, "on": "yes"}),
    ] {
        let handled = handlers().handle(&request(SAVE, &fields));
        assert_eq!(failure_of(&handled).0, ErrorCode::Invalid, "{fields}");
        assert_eq!(handled.error_fields, None);
    }
}

#[test]
fn a_taken_name_is_a_name_rule() {
    let handlers = handlers();
    saved_id(&handlers, "Nightly");
    let handled = handlers.handle(&request(SAVE, &save_fields("nightly")));
    assert_eq!(
        handled.error_fields.expect("fields")[0]["rule"],
        "name_taken"
    );
}

#[test]
fn the_cap_refusal_names_the_cap() {
    let handlers = handlers_with(1, Arc::new(RecordingRunNow::default()));
    saved_id(&handlers, "One");
    let handled = handlers.handle(&request(SAVE, &save_fields("Two")));
    let (code, message) = failure_of(&handled);
    assert_eq!(code, ErrorCode::Refused);
    assert!(message.contains("schedule cap"));
    assert!(message.contains('1'));
}

#[test]
fn a_switch_on_of_a_future_once_time_is_accepted() {
    let handlers = handlers();
    let mut fields = save_fields("Once");
    fields["repeat"] = json!({"kind": "once", "date": "2026-06-01", "time": "15:00"});
    fields["on"] = json!(false);
    let id = result_of(&handlers.handle(&request(SAVE, &fields)))["schedule"]["id"]
        .as_str()
        .expect("id")
        .to_owned();
    // 15:00 Berlin on 2026-06-01 is 13:00 UTC, an hour after NOW: still future.
    let on = handlers.handle(&request(SWITCH, &json!({"schedule_id": id, "on": true})));
    assert!(matches!(on.response, Response::Success { .. }));
}

#[test]
fn a_switch_on_for_a_once_time_that_passed_is_refused() {
    let handlers = handlers();
    let mut fields = save_fields("Once");
    fields["repeat"] = json!({"kind": "once", "date": "2026-05-31", "time": "15:00"});
    fields["on"] = json!(false);
    let id = result_of(&handlers.handle(&request(SAVE, &fields)))["schedule"]["id"]
        .as_str()
        .expect("id")
        .to_owned();
    let on = handlers.handle(&request(SWITCH, &json!({"schedule_id": id, "on": true})));
    assert_eq!(failure_of(&on).0, ErrorCode::Refused);
}

#[test]
fn an_unknown_schedule_is_not_found_and_a_bad_id_is_invalid() {
    let handlers = handlers();
    let unknown = ScheduleId::from_u128(9).to_string();
    for kind in [SWITCH, RUN_NOW, DELETE, HISTORY] {
        let handled = handlers.handle(&request(kind, &json!({"schedule_id": unknown, "on": true})));
        assert_eq!(failure_of(&handled).0, ErrorCode::NotFound, "{kind}");
    }
    let bad = handlers.handle(&request(DELETE, &json!({"schedule_id": "nope"})));
    assert_eq!(failure_of(&bad).0, ErrorCode::Invalid);
}

#[test]
fn delete_answers_and_announces_the_removal() {
    let handlers = handlers();
    let id = saved_id(&handlers, "Nightly");
    let handled = handlers.handle(&request(DELETE, &json!({"schedule_id": id})));
    assert_eq!(result_of(&handled)["schedule_id"], id);
    assert!(
        handled.events[0]
            .encode()
            .contains("\"type\":\"schedule.removed\"")
    );
    assert!(handled.events[0].encode().contains(&id));
}

#[test]
fn history_lists_the_runs() {
    let handlers = handlers();
    let id = saved_id(&handlers, "Nightly");
    let handled = handlers.handle(&request(HISTORY, &json!({"schedule_id": id})));
    assert_eq!(result_of(&handled)["runs"], json!([]));
}

#[test]
fn run_now_hands_the_schedule_to_the_host() {
    let run_now = Arc::new(RecordingRunNow::default());
    let handlers = handlers_with(50, run_now.clone());
    let id = saved_id(&handlers, "Nightly");
    let handled = handlers.handle(&request(RUN_NOW, &json!({"schedule_id": id})));
    assert!(matches!(handled.response, Response::Success { .. }));
    assert_eq!(run_now.0.lock().expect("lock").len(), 1);
}

#[test]
fn a_run_now_that_the_host_refuses_is_refused() {
    let handlers = handlers_with(50, Arc::new(RefusingRunNow));
    let id = saved_id(&handlers, "Nightly");
    let handled = handlers.handle(&request(RUN_NOW, &json!({"schedule_id": id})));
    assert_eq!(
        failure_of(&handled),
        (ErrorCode::Refused, "the office is on hold")
    );
}

#[test]
fn a_request_of_another_type_is_not_found() {
    let handled = handlers().handle(&request("schedule.nothing", &json!({})));
    assert_eq!(failure_of(&handled).0, ErrorCode::NotFound);
}
