// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! Scheduled directives (0025) through a real server with fake CLIs. The
//! schedules read a fake clock and a fake time zone that the test moves, so
//! an hour passes in a moment. Each test is one success criterion or one edge
//! case of the spec.

mod support;

use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use agent_infrastructure::persistence::SharedConnection;
use agent_infrastructure::services::find_program;
use hierarchy_application::commands::place_agent::{PlaceAgentHandler, PlaceAgentRequest};
use hierarchy_application::common::interfaces::HierarchyStore;
use hierarchy_domain::Role;
use hierarchy_infrastructure::persistence::repositories::hierarchy_repository::HierarchyRepository;
use hierarchy_infrastructure::services::agent_places::AgentPlaces;
use jiff::tz::TimeZone;
use kernel::Timestamp;
use office_harnessd::bridges::ScheduleTime;
use office_harnessd::server::ServerOptions;
use serde_json::{Value, json};
use support::fake_cli::FakeClis;
use support::office_client::{Client, Office, error_text, wait_for};
use support::running_office::serve_again_with;

const MINUTE: i64 = 60_000;
const HOUR: i64 = 3_600_000;
const SECOND: i64 = 1_000;
/// Fifty milliseconds a round keep the tests short.
const TICK: Duration = Duration::from_millis(50);
const CHECK_PERIOD: Duration = Duration::from_secs(2);
const RESULT_LIMIT: Duration = Duration::from_secs(60);
const POLL: Duration = Duration::from_millis(50);
/// Long enough for several rounds, so a run that would start has started.
const SEVERAL_ROUNDS: Duration = Duration::from_millis(400);

/// A clock and a zone that the test sets.
#[derive(Debug)]
struct FakeTime {
    millis: AtomicI64,
    zone: Mutex<TimeZone>,
}

impl FakeTime {
    /// The server keeps a reference for as long as it runs, so the fake lives
    /// for the process.
    fn leak(start: i64) -> &'static Self {
        Box::leak(Box::new(Self {
            millis: AtomicI64::new(start),
            zone: Mutex::new(TimeZone::UTC),
        }))
    }

    fn now_millis(&self) -> i64 {
        self.millis.load(Ordering::SeqCst)
    }

    fn set(&self, millis: i64) {
        self.millis.store(millis, Ordering::SeqCst);
    }

    fn advance(&self, millis: i64) {
        self.millis.fetch_add(millis, Ordering::SeqCst);
    }

    fn set_zone(&self, name: &str) {
        *self.zone.lock().expect("zone") = TimeZone::get(name).expect("a zone of the database");
    }
}

impl ScheduleTime for FakeTime {
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_millis(self.now_millis())
    }

    fn zone(&self) -> TimeZone {
        self.zone.lock().expect("zone").clone()
    }
}

/// Monday 2026-03-02 10:00 UTC.
fn start_of_day() -> i64 {
    "2026-03-02T10:00:00Z"
        .parse::<jiff::Timestamp>()
        .expect("a timestamp")
        .as_millisecond()
}

fn once_at(millis: i64) -> Value {
    let moment = jiff::Timestamp::from_millisecond(millis)
        .expect("a moment")
        .to_zoned(TimeZone::UTC);
    json!({
        "kind": "once",
        "date": moment.date().to_string(),
        "time": format!("{:02}:{:02}", moment.hour(), moment.minute()),
    })
}

fn every_hours(hours: u32) -> Value {
    json!({"kind": "every_hours", "hours": hours})
}

fn options(time: &'static FakeTime) -> ServerOptions {
    ServerOptions {
        schedule_tick: TICK,
        schedule_time: Some(time),
        resume_check_period: CHECK_PERIOD,
        ..ServerOptions::default()
    }
}

/// An office with a fake clock, and a client of the person.
struct World {
    office: Office,
    client: Client,
    time: &'static FakeTime,
}

impl World {
    async fn open() -> Self {
        let time = FakeTime::leak(start_of_day());
        Self::open_with(FakeClis::new(), options(time), time).await
    }

    async fn open_with(clis: FakeClis, options: ServerOptions, time: &'static FakeTime) -> Self {
        let office = Office::open_with_options(clis, options).await;
        let client = office.client().await;
        Self {
            office,
            client,
            time,
        }
    }

    fn records(&self) -> PathBuf {
        self.office.records_path().to_path_buf()
    }

    /// What the fake CLI of `name` received from the terminal.
    fn typed(&self, name: &str) -> String {
        fs::read_to_string(self.records().join(format!("typed-{name}"))).unwrap_or_default()
    }

    /// The body of a CLI that shows a prompt and keeps what is typed to it.
    fn typing_body(&self, name: &str) -> String {
        format!(
            "echo ready\nstty raw echo\ncat >> {}/typed-{name}",
            self.records().display()
        )
    }

    /// An agent that waits for input and keeps what it is typed.
    async fn ready_agent(&mut self, name: &str) -> String {
        let body = self.typing_body(name);
        self.agent_running(name, &body).await
    }

    async fn agent_running(&mut self, name: &str, body: &str) -> String {
        let command = format!("cli-{name}");
        self.office.clis.add(&command, body);
        let agent = self
            .client
            .create_agent(&self.office, name, &command, None)
            .await;
        self.client.start(&agent).await;
        self.client
            .wait_for_status(&agent, "waiting_for_input")
            .await;
        agent
    }

    async fn save(
        &mut self,
        name: &str,
        target: &str,
        text: &str,
        repeat: Value,
        on: bool,
    ) -> Value {
        self.client
            .ask(
                "schedule.save",
                json!({
                    "name": name,
                    "text": text,
                    "target_agent_id": target,
                    "repeat": repeat,
                    "on": on,
                }),
            )
            .await
    }

    /// Saves a schedule that must be kept, and returns its id.
    async fn saved(&mut self, name: &str, target: &str, text: &str, repeat: Value) -> String {
        let answer = self.save(name, target, text, repeat, true).await;
        assert_eq!(answer["ok"], true, "{answer}");
        answer["result"]["schedule"]["id"]
            .as_str()
            .expect("id")
            .to_owned()
    }

    async fn schedule(&mut self, id: &str) -> Value {
        let listed = self.client.ask("schedule.list", json!({})).await;
        listed["result"]["schedules"]
            .as_array()
            .expect("schedules")
            .iter()
            .find(|schedule| schedule["id"] == id)
            .cloned()
            .unwrap_or(Value::Null)
    }

    async fn history(&mut self, id: &str) -> Vec<Value> {
        let answer = self
            .client
            .ask("schedule.history", json!({"schedule_id": id}))
            .await;
        answer["result"]["runs"]
            .as_array()
            .unwrap_or(&Vec::new())
            .clone()
    }

    /// Waits until the history holds `count` rows that have a result.
    async fn wait_for_runs(&mut self, id: &str, count: usize) -> Vec<Value> {
        let deadline = Instant::now() + RESULT_LIMIT;
        loop {
            let runs: Vec<Value> = self
                .history(id)
                .await
                .into_iter()
                .filter(|run| !run["result"].is_null())
                .collect();
            if runs.len() >= count {
                return runs;
            }
            assert!(Instant::now() < deadline, "{id}: {count} runs: {runs:?}");
            tokio::time::sleep(POLL).await;
        }
    }

    /// Waits for the newest row with `result`.
    async fn wait_for_result(&mut self, id: &str, result: &str) -> Value {
        let deadline = Instant::now() + RESULT_LIMIT;
        loop {
            let runs = self.history(id).await;
            if let Some(run) = runs.iter().find(|run| run["result"] == result) {
                return run.clone();
            }
            assert!(Instant::now() < deadline, "{id}: no {result}: {runs:?}");
            tokio::time::sleep(POLL).await;
        }
    }

    async fn wait_for_typed(&mut self, name: &str, text: &str) {
        let deadline = Instant::now() + RESULT_LIMIT;
        while !self.typed(name).contains(text) {
            assert!(Instant::now() < deadline, "{name} never got {text}");
            tokio::time::sleep(POLL).await;
        }
    }

    async fn actions_of(&mut self, agent: &str) -> Vec<Value> {
        let answer = self
            .client
            .ask("agent.actions", json!({"agent_id": agent}))
            .await;
        answer["result"].as_array().expect("entries").clone()
    }

    /// The entries of the target's record that have this schedule as actor.
    async fn entries_of_schedule(&mut self, agent: &str, schedule: &str) -> Vec<Value> {
        let actor = format!("schedule:{schedule}");
        self.actions_of(agent)
            .await
            .into_iter()
            .filter(|entry| entry["actor"] == actor)
            .collect()
    }

    /// The notifications of the schedule event.
    async fn schedule_notifications(&mut self) -> Vec<Value> {
        let listed = self.client.ask("notification.list", json!({})).await;
        listed["result"]
            .as_array()
            .expect("notifications")
            .iter()
            .filter(|notification| notification["event"] == "schedule")
            .cloned()
            .collect()
    }

    /// Boxed, because the world is large and the lint for large futures
    /// would otherwise ask for a box at each call.
    fn stop(self) -> Later<()> {
        Box::pin(async move {
            self.office.serving.stop().await;
        })
    }
}

type Later<T> = Pin<Box<dyn Future<Output = T>>>;

/// Stops the server and starts it again on the same folder, as a restart does.
fn restart(world: World, time: &'static FakeTime) -> Later<World> {
    Box::pin(async move {
        let stopped = Box::pin(restart_without_start(world)).await;
        Box::pin(stopped.start_again(time)).await
    })
}

#[tokio::test]
async fn a_once_schedule_types_its_text_at_the_due_time_and_names_itself_in_the_record() {
    let mut world = World::open().await;
    let agent = world.ready_agent("Ana").await;
    let due = world.time.now_millis() + 3 * MINUTE;

    let saved = world
        .save("smoke", &agent, "print hello", once_at(due), true)
        .await;

    assert_eq!(saved["ok"], true, "{saved}");
    let id = saved["result"]["schedule"]["id"].as_str().expect("id");
    assert_eq!(saved["result"]["schedule"]["next_due"], due);
    let listed = world.client.ask("schedule.list", json!({})).await;
    assert_eq!(listed["result"]["zone"], "UTC", "{listed}");

    world.time.advance(3 * MINUTE + SECOND);
    let run = world.wait_for_result(id, "submitted").await;
    assert_eq!(run["via"], "due");
    assert_eq!(run["due"], due);
    world.wait_for_typed("Ana", "print hello\r").await;
    let entries = world.entries_of_schedule(&agent, id).await;
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0]["action"], "directive");
    assert_eq!(entries[0]["result"], "ok");
    assert_eq!(entries[0]["actor_label"], "smoke");
    let after = world.schedule(id).await;
    assert_eq!(after["on"], false, "a once schedule ends: {after}");
    assert_eq!(after["next_due"], Value::Null);
    assert_eq!(after["off_reason"], Value::Null);
    world.stop().await;
}

#[tokio::test]
async fn each_save_switch_run_now_and_delete_of_the_person_adds_one_entry_with_the_person() {
    let mut world = World::open().await;
    let agent = world.ready_agent("Ana").await;
    let id = world.saved("nightly", &agent, "go", every_hours(4)).await;
    world
        .client
        .ask("schedule.switch", json!({"schedule_id": id, "on": false}))
        .await;
    world
        .client
        .ask("schedule.run_now", json!({"schedule_id": id}))
        .await;
    world.wait_for_result(&id, "submitted").await;
    world
        .client
        .ask("schedule.delete", json!({"schedule_id": id}))
        .await;

    let entries = world.actions_of(&agent).await;
    let by_person: Vec<&str> = entries
        .iter()
        .filter(|entry| entry["actor"] == "user")
        .filter_map(|entry| entry["detail"].as_str())
        .filter(|detail| detail.contains("schedule"))
        .collect();
    assert_eq!(by_person.len(), 4, "{entries:?}");
    for verb in ["created", "switched off", "run now", "deleted"] {
        assert!(
            by_person.iter().any(|detail| detail.contains(verb)),
            "{verb}: {by_person:?}"
        );
    }
    world.stop().await;
}

#[tokio::test]
async fn a_server_stop_over_due_times_makes_no_late_run_and_gives_one_missed_entry_and_one_notice()
{
    let time = FakeTime::leak(start_of_day());
    let mut world = World::open_with(FakeClis::new(), options(time), time).await;
    let agent = world.ready_agent("Ana").await;
    let hourly = world.saved("hourly", &agent, "tick", every_hours(1)).await;
    let once_id = world
        .saved(
            "once",
            &agent,
            "tick",
            once_at(start_of_day() + 150 * MINUTE),
        )
        .await;
    time.advance(30 * MINUTE);
    tokio::time::sleep(SEVERAL_ROUNDS).await;

    let mut world = restart(world, time).await;
    // The machine was off from 10:30 to 13:40.
    time.set(start_of_day() + 220 * MINUTE);
    world.client.start(&agent).await;
    world
        .client
        .wait_for_status(&agent, "waiting_for_input")
        .await;

    let missed = world.wait_for_result(&hourly, "missed").await;
    assert!(
        missed["reason"]
            .as_str()
            .expect("reason")
            .starts_with("3 runs missed, first 2026-03-02 11:00, last 2026-03-02 13:00"),
        "{missed}"
    );
    let once_missed = world.wait_for_result(&once_id, "missed").await;
    assert!(
        once_missed["reason"]
            .as_str()
            .expect("reason")
            .contains("1 run missed")
    );
    let off = world.schedule(&once_id).await;
    assert_eq!(off["on"], false);
    assert_eq!(off["off_reason"], "time_missed");
    let next = world.schedule(&hourly).await;
    assert_eq!(next["next_due"], start_of_day() + 4 * HOUR);
    assert_eq!(world.typed("Ana"), "", "no late run");
    let notices = world.schedule_notifications().await;
    assert_eq!(notices.len(), 1, "{notices:?}");
    assert_eq!(notices[0]["agent_name"], "2 schedules missed runs");

    time.set(start_of_day() + 4 * HOUR + 10 * SECOND);
    world.wait_for_result(&hourly, "submitted").await;
    world.wait_for_typed("Ana", "tick\r").await;
    world.stop().await;
}

#[tokio::test]
async fn a_run_started_90_seconds_late_runs_and_one_started_150_seconds_late_is_missed() {
    let mut world = World::open().await;
    let agent = world.ready_agent("Ana").await;
    let late = world.saved("late", &agent, "in time", every_hours(1)).await;
    let later = world
        .saved("later", &agent, "too late", every_hours(2))
        .await;

    // One step over the due times, as a server that a sweep or a sleep kept.
    world.time.advance(HOUR + 90 * SECOND);
    world.wait_for_result(&late, "submitted").await;
    world.wait_for_typed("Ana", "in time\r").await;
    world.time.advance(HOUR + 60 * SECOND);
    let missed = world.wait_for_result(&later, "missed").await;

    assert!(
        missed["reason"]
            .as_str()
            .expect("reason")
            .starts_with("1 run missed")
    );
    assert!(!world.typed("Ana").contains("too late"));
    world.stop().await;
}

#[tokio::test]
async fn a_clock_set_back_after_a_run_does_not_run_that_due_time_again() {
    let mut world = World::open().await;
    let agent = world.ready_agent("Ana").await;
    let id = world.saved("hourly", &agent, "once", every_hours(1)).await;
    world.time.set(start_of_day() + HOUR + 10 * SECOND);
    world.wait_for_runs(&id, 1).await;

    world.time.set(start_of_day() + 30 * MINUTE);
    tokio::time::sleep(SEVERAL_ROUNDS).await;
    world.time.set(start_of_day() + HOUR + 20 * SECOND);
    tokio::time::sleep(SEVERAL_ROUNDS).await;

    assert_eq!(world.history(&id).await.len(), 1);
    world.wait_for_typed("Ana", "once\r").await;
    assert_eq!(world.typed("Ana").matches("once").count(), 1);
    world.stop().await;
}

#[tokio::test]
async fn a_clock_set_three_hours_forward_makes_one_missed_entry_with_the_count_3_and_no_run() {
    let mut world = World::open().await;
    let agent = world.ready_agent("Ana").await;
    let id = world.saved("hourly", &agent, "never", every_hours(1)).await;

    world.time.advance(3 * HOUR + 10 * MINUTE);
    let runs = world.wait_for_runs(&id, 1).await;

    assert_eq!(runs.len(), 1, "{runs:?}");
    assert_eq!(runs[0]["result"], "missed");
    assert!(
        runs[0]["reason"]
            .as_str()
            .expect("reason")
            .starts_with("3 runs missed")
    );
    assert_eq!(world.typed("Ana"), "");
    world.stop().await;
}

#[tokio::test]
async fn a_daily_schedule_follows_a_change_of_the_time_zone() {
    let mut world = World::open().await;
    let agent = world.ready_agent("Ana").await;
    let saved = world
        .save(
            "daily",
            &agent,
            "morning",
            json!({"kind": "daily", "time": "09:00"}),
            true,
        )
        .await;
    let id = saved["result"]["schedule"]["id"]
        .as_str()
        .expect("id")
        .to_owned();
    // 10:00 UTC on Monday: 09:00 UTC on Tuesday is next.
    let first = start_of_day() + 23 * HOUR;
    assert_eq!(saved["result"]["schedule"]["next_due"], first);

    // Tokyo is UTC+9, where it is 19:00 Monday: 09:00 JST on Tuesday is 00:00 UTC.
    world.time.set_zone("Asia/Tokyo");
    let deadline = Instant::now() + RESULT_LIMIT;
    let next = start_of_day() + 14 * HOUR;
    while world.schedule(&id).await["next_due"] != next {
        assert!(
            Instant::now() < deadline,
            "the next due time never followed the zone"
        );
        tokio::time::sleep(POLL).await;
    }
    let listed = world.client.ask("schedule.list", json!({})).await;
    assert_eq!(listed["result"]["zone"], "Asia/Tokyo");

    world.time.set(next + 10 * SECOND);
    world.wait_for_result(&id, "submitted").await;
    world.wait_for_typed("Ana", "morning\r").await;
    world.stop().await;
}

#[tokio::test]
async fn under_an_office_hold_each_due_run_is_refused_recorded_and_never_made_later() {
    let mut world = World::open().await;
    let agent = world.ready_agent("Ana").await;
    let id = world.saved("hourly", &agent, "held", every_hours(1)).await;
    world.client.ask("hold.set", json!({})).await;

    for count in 1..=3 {
        world.time.advance(HOUR + SECOND);
        let runs = world.wait_for_runs(&id, count).await;
        assert_eq!(runs[0]["result"], "refused", "{runs:?}");
        assert!(
            runs[0]["reason"].as_str().expect("reason").contains("hold"),
            "{runs:?}"
        );
    }
    let refused: Vec<Value> = world
        .entries_of_schedule(&agent, &id)
        .await
        .into_iter()
        .filter(|entry| entry["action"] == "refused")
        .collect();
    assert_eq!(refused.len(), 3, "{refused:?}");

    world.client.ask("hold.release", json!({})).await;
    tokio::time::sleep(SEVERAL_ROUNDS).await;
    assert_eq!(world.history(&id).await.len(), 3, "no catch-up run");
    assert_eq!(world.typed("Ana"), "");
    let holds = world.client.ask("hold.list", json!({})).await;
    assert_eq!(holds["result"]["agents"], json!([]), "{holds}");
    assert_eq!(world.schedule_notifications().await, Vec::<Value>::new());
    world.stop().await;
}

#[tokio::test]
async fn under_a_hold_on_the_target_each_due_run_is_refused_with_the_hold() {
    let mut world = World::open().await;
    let agent = world.ready_agent("Ana").await;
    let id = world.saved("hourly", &agent, "held", every_hours(1)).await;
    world
        .client
        .ask("hold.set", json!({"agent_id": agent}))
        .await;

    for count in 1..=2 {
        world.time.advance(HOUR + SECOND);
        let runs = world.wait_for_runs(&id, count).await;
        assert_eq!(runs[0]["result"], "refused", "{runs:?}");
    }

    assert_eq!(world.typed("Ana"), "");
    assert_eq!(world.schedule_notifications().await, Vec::<Value>::new());
    world.stop().await;
}

#[tokio::test]
async fn the_fourth_schedule_for_one_target_in_a_minute_is_refused_with_the_limit_and_its_value() {
    let mut world = World::open().await;
    let agent = world.ready_agent("Ana").await;
    world
        .client
        .set_setting("limit_directives_per_target", "3")
        .await;
    let mut ids = Vec::new();
    for number in 1..=4 {
        let name = format!("s{number}");
        ids.push(world.saved(&name, &agent, &name, every_hours(1)).await);
    }

    world.time.advance(HOUR + SECOND);
    let mut results = Vec::new();
    for id in &ids {
        let runs = world.wait_for_runs(id, 1).await;
        results.push(runs[0].clone());
    }

    let refused: Vec<&Value> = results
        .iter()
        .filter(|run| run["result"] == "refused")
        .collect();
    assert_eq!(refused.len(), 1, "{results:?}");
    assert!(refused[0]["reason"].as_str().expect("reason").contains('3'));
    for run in results.iter().filter(|run| run["result"] != "refused") {
        assert!(
            run["result"] == "submitted" || run["result"] == "waiting",
            "{run}"
        );
    }
    let holds = world.client.ask("hold.list", json!({})).await;
    assert_eq!(holds["result"]["agents"], json!([]), "{holds}");
    world.stop().await;
}

#[tokio::test]
async fn an_asleep_target_is_woken_by_the_schedule_and_then_gets_the_directive() {
    let time = FakeTime::leak(start_of_day());
    let mut world = World::open_with(
        FakeClis::new(),
        ServerOptions {
            idle_sleep_tick: TICK,
            idle_sleep_minute: Duration::from_millis(100),
            ..options(time)
        },
        time,
    )
    .await;
    let records = world.records();
    let tail = format!(
        "echo ready; stty raw echo; cat >> {}/typed-Ana",
        records.display()
    );
    let body = format!(
        "cd {}\nif [ ! -e first-run-done ]; then : > first-run-done; echo ready; sleep 30; fi\n{tail}",
        records.display()
    );
    world.office.clis.add("claude", &body);
    let agent = world
        .client
        .create_agent(&world.office, "Ana", "claude", None)
        .await;
    world.client.start(&agent).await;
    world
        .client
        .wait_for_status(&agent, "waiting_for_input")
        .await;
    world.client.dismiss_needs_you(&agent).await;
    world.client.set_setting("idle_sleep_minutes", "5").await;
    world.client.wait_for_status(&agent, "asleep").await;
    // The woken agent must not fall asleep again.
    world.client.set_setting("idle_sleep_minutes", "1440").await;
    let id = world.saved("wake", &agent, "wake up", every_hours(1)).await;

    world.time.advance(HOUR + SECOND);
    world.wait_for_result(&id, "submitted").await;

    world.wait_for_typed("Ana", "wake up\r").await;
    let wakes: Vec<Value> = world
        .entries_of_schedule(&agent, &id)
        .await
        .into_iter()
        .filter(|entry| entry["action"] == "wake")
        .collect();
    assert_eq!(wakes.len(), 1, "{wakes:?}");
    assert_eq!(wakes[0]["actor_label"], "wake");
    world.stop().await;
}

#[tokio::test]
async fn a_stopped_target_gives_skipped_runs_one_notice_and_a_new_one_after_a_success() {
    let mut world = World::open().await;
    let agent = world.ready_agent("Ana").await;
    world
        .client
        .ask("agent.stop", json!({"agent_id": agent}))
        .await;
    world.client.wait_for_status(&agent, "stopped").await;
    let id = world.saved("hourly", &agent, "hello", every_hours(1)).await;

    for count in 1..=5 {
        world.time.advance(HOUR + SECOND);
        let runs = world.wait_for_runs(&id, count).await;
        assert_eq!(runs[0]["result"], "skipped", "{runs:?}");
        assert_eq!(runs[0]["reason"], "stopped", "{runs:?}");
    }
    assert_eq!(world.schedule_notifications().await.len(), 1);
    assert_eq!(world.typed("Ana"), "", "nothing started");

    world.client.start(&agent).await;
    world
        .client
        .wait_for_status(&agent, "waiting_for_input")
        .await;
    world.time.advance(HOUR + SECOND);
    world.wait_for_result(&id, "submitted").await;
    world
        .client
        .ask("agent.stop", json!({"agent_id": agent}))
        .await;
    world.client.wait_for_status(&agent, "stopped").await;
    world.time.advance(HOUR + SECOND);
    world.wait_for_runs(&id, 7).await;

    assert_eq!(world.schedule_notifications().await.len(), 2);
    world.stop().await;
}

#[tokio::test]
async fn deleting_the_target_switches_the_schedule_off_at_once_with_one_notice_and_no_run() {
    let mut world = World::open().await;
    let agent = world.ready_agent("Ana").await;
    let id = world.saved("hourly", &agent, "gone", every_hours(1)).await;

    let answer = world
        .client
        .ask("agent.delete", json!({"agent_id": agent}))
        .await;
    assert_eq!(answer["ok"], true, "{answer}");

    let after = world.schedule(&id).await;
    assert_eq!(after["on"], false, "{after}");
    assert_eq!(after["off_reason"], "target_deleted");
    assert_eq!(after["next_due"], Value::Null);
    assert_eq!(world.schedule_notifications().await.len(), 1);
    world.time.advance(HOUR + SECOND);
    tokio::time::sleep(SEVERAL_ROUNDS).await;
    assert_eq!(
        world.history(&id).await,
        Vec::<Value>::new(),
        "no run follows"
    );
    let switched = world
        .client
        .ask("schedule.switch", json!({"schedule_id": id, "on": true}))
        .await;
    assert_eq!(error_text(&switched).0, "refused", "{switched}");
    let now = world
        .client
        .ask("schedule.run_now", json!({"schedule_id": id}))
        .await;
    assert_eq!(error_text(&now).0, "refused", "{now}");
    world.stop().await;
}

#[tokio::test]
async fn run_now_twice_waits_the_first_and_skips_the_second_for_the_previous_run() {
    let mut world = World::open().await;
    let flag = world.records().join("go");
    let body = format!(
        "while [ ! -e {flag} ]; do echo tick; sleep 0.1; done\n{}",
        world.typing_body("Ana"),
        flag = flag.display()
    );
    let command = "cli-Ana";
    world.office.clis.add(command, &body);
    let agent = world
        .client
        .create_agent(&world.office, "Ana", command, None)
        .await;
    world.client.start(&agent).await;
    world.client.wait_for_status(&agent, "working").await;
    let id = world.saved("busy", &agent, "later", every_hours(24)).await;

    world
        .client
        .ask("schedule.run_now", json!({"schedule_id": id}))
        .await;
    world
        .client
        .ask("schedule.run_now", json!({"schedule_id": id}))
        .await;

    let waiting = world.wait_for_result(&id, "waiting").await;
    assert_eq!(waiting["via"], "now");
    let skipped = world.wait_for_result(&id, "skipped").await;
    assert_eq!(skipped["reason"], "previous run still waiting");
    let schedule = world.schedule(&id).await;
    assert_eq!(schedule["on"], true, "Run now leaves the switch");
    fs::write(flag, "").expect("flag");
    world.wait_for_result(&id, "submitted").await;
    world.wait_for_typed("Ana", "later\r").await;
    world.stop().await;
}

#[tokio::test]
async fn a_delete_of_a_schedule_whose_directive_waits_leaves_the_directive_to_be_typed() {
    let mut world = World::open().await;
    let flag = world.records().join("go");
    let body = format!(
        "while [ ! -e {} ]; do echo tick; sleep 0.1; done\n{}",
        flag.display(),
        world.typing_body("Ana"),
    );
    world.office.clis.add("cli-Ana", &body);
    let agent = world
        .client
        .create_agent(&world.office, "Ana", "cli-Ana", None)
        .await;
    world.client.start(&agent).await;
    world.client.wait_for_status(&agent, "working").await;
    let id = world
        .saved("busy", &agent, "kept text", every_hours(24))
        .await;
    world
        .client
        .ask("schedule.run_now", json!({"schedule_id": id}))
        .await;
    world.wait_for_result(&id, "waiting").await;

    let deleted = world
        .client
        .ask("schedule.delete", json!({"schedule_id": id}))
        .await;
    assert_eq!(deleted["ok"], true, "{deleted}");
    fs::write(flag, "").expect("flag");

    world.wait_for_typed("Ana", "kept text\r").await;
    assert_eq!(world.schedule(&id).await, Value::Null);
    let deadline = Instant::now() + RESULT_LIMIT;
    loop {
        let entries = world.entries_of_schedule(&agent, &id).await;
        if entries.iter().any(|entry| entry["result"] == "ok") {
            assert_eq!(entries[0]["actor_label"], "busy", "{entries:?}");
            break;
        }
        assert!(Instant::now() < deadline, "no entry of the submitted run");
        tokio::time::sleep(POLL).await;
    }
    world.stop().await;
}

#[tokio::test]
async fn an_edit_while_the_directive_waits_leaves_the_waiting_text_and_changes_the_next_due_time() {
    let mut world = World::open().await;
    let flag = world.records().join("go");
    let body = format!(
        "while [ ! -e {} ]; do echo tick; sleep 0.1; done\n{}",
        flag.display(),
        world.typing_body("Ana"),
    );
    world.office.clis.add("cli-Ana", &body);
    let agent = world
        .client
        .create_agent(&world.office, "Ana", "cli-Ana", None)
        .await;
    world.client.start(&agent).await;
    world.client.wait_for_status(&agent, "working").await;
    let id = world
        .saved("busy", &agent, "old text", every_hours(24))
        .await;
    world
        .client
        .ask("schedule.run_now", json!({"schedule_id": id}))
        .await;
    world.wait_for_result(&id, "waiting").await;

    let edited = world
        .client
        .ask(
            "schedule.save",
            json!({
                "schedule_id": id,
                "name": "busy",
                "text": "new text",
                "target_agent_id": agent,
                "repeat": every_hours(2),
                "on": true,
            }),
        )
        .await;
    assert_eq!(edited["ok"], true, "{edited}");
    assert_eq!(
        edited["result"]["schedule"]["next_due"],
        world.time.now_millis() + 2 * HOUR
    );
    fs::write(flag, "").expect("flag");

    world.wait_for_typed("Ana", "old text\r").await;
    assert!(!world.typed("Ana").contains("new text"));
    world.stop().await;
}

#[tokio::test]
async fn an_unsubmitted_directive_is_recorded_once_in_the_targets_record_and_notified() {
    let mut world = World::open().await;
    // No echo: the typed text never shows at the input line.
    let body = format!(
        "echo ready\nstty raw -echo\ncat >> {}/typed-Ana",
        world.records().display()
    );
    let agent = world.agent_running("Ana", &body).await;
    let id = world.saved("quiet", &agent, "unseen", every_hours(1)).await;

    world.time.advance(HOUR + SECOND);
    let run = world.wait_for_result(&id, "unsubmitted").await;

    assert!(
        run["reason"]
            .as_str()
            .expect("reason")
            .contains("input line")
    );
    let entries = world.entries_of_schedule(&agent, &id).await;
    let failures: Vec<&Value> = entries
        .iter()
        .filter(|entry| entry["result"] != "ok")
        .collect();
    assert_eq!(failures.len(), 1, "{entries:?}");
    assert_eq!(failures[0]["actor_label"], "quiet");
    assert_eq!(world.schedule_notifications().await.len(), 1);
    world.stop().await;
}

#[tokio::test]
async fn a_waiting_directive_whose_target_stops_is_withdrawn_and_recorded_once() {
    let mut world = World::open().await;
    let body = "while true; do echo tick; sleep 0.1; done";
    let agent = world.agent_running_working("Ana", body).await;
    let id = world.saved("busy", &agent, "pending", every_hours(1)).await;
    world.time.advance(HOUR + SECOND);
    world.wait_for_result(&id, "waiting").await;

    world
        .client
        .ask("agent.stop", json!({"agent_id": agent}))
        .await;

    let run = world.wait_for_result(&id, "withdrawn").await;
    assert!(run["reason"].is_string(), "{run}");
    let deadline = Instant::now() + RESULT_LIMIT;
    loop {
        let entries = world.entries_of_schedule(&agent, &id).await;
        let failures = entries
            .iter()
            .filter(|entry| entry["result"] != "ok")
            .count();
        if failures >= 1 {
            assert_eq!(failures, 1, "{entries:?}");
            break;
        }
        assert!(Instant::now() < deadline, "no entry of the withdrawal");
        tokio::time::sleep(POLL).await;
    }
    world.stop().await;
}

impl World {
    /// An agent whose CLI never goes quiet, so it shows `working`.
    async fn agent_running_working(&mut self, name: &str, body: &str) -> String {
        let command = format!("cli-{name}");
        self.office.clis.add(&command, body);
        let agent = self
            .client
            .create_agent(&self.office, name, &command, None)
            .await;
        self.client.start(&agent).await;
        self.client.wait_for_status(&agent, "working").await;
        agent
    }
}

#[tokio::test]
async fn the_cap_refuses_the_next_save_with_its_value_and_a_cap_of_200_takes_200() {
    let mut world = World::open().await;
    let agent = world.ready_agent("Ana").await;
    world.client.set_setting("schedule_cap", "3").await;
    for number in 1..=3 {
        world
            .saved(&format!("s{number}"), &agent, "x", every_hours(24))
            .await;
    }

    let refused = world.save("s4", &agent, "x", every_hours(24), true).await;

    assert_eq!(error_text(&refused).0, "refused", "{refused}");
    assert!(error_text(&refused).1.contains('3'), "{refused}");
    let listed = world.client.ask("schedule.list", json!({})).await;
    assert_eq!(
        listed["result"]["schedules"]
            .as_array()
            .expect("list")
            .len(),
        3
    );

    world.client.set_setting("schedule_cap", "200").await;
    for number in 4..=200 {
        world
            .saved(&format!("s{number}"), &agent, "x", every_hours(24))
            .await;
    }
    let listed = world.client.ask("schedule.list", json!({})).await;
    assert_eq!(
        listed["result"]["schedules"]
            .as_array()
            .expect("list")
            .len(),
        200
    );
    world.stop().await;
}

#[tokio::test]
async fn a_cap_outside_1_to_200_is_refused_and_the_old_value_stays() {
    let mut world = World::open().await;
    for value in ["0", "201", "2.5", "many"] {
        let answer = world
            .client
            .ask(
                "settings.set",
                json!({"key": "schedule_cap", "value": value}),
            )
            .await;
        assert_eq!(answer["ok"], false, "{value}: {answer}");
    }
    let stored = world
        .client
        .ask("settings.get", json!({"key": "schedule_cap"}))
        .await;
    assert_eq!(stored["result"]["value"], "50", "{stored}");
    world.stop().await;
}

#[tokio::test]
async fn a_save_that_breaks_rules_names_each_field_and_rule_in_the_error() {
    let mut world = World::open().await;
    let agent = world.ready_agent("Ana").await;

    let answer = world
        .save(
            " ",
            &agent,
            "text\u{1b}[D",
            json!({"kind": "weekly", "days": [], "time": "09:00"}),
            true,
        )
        .await;

    assert_eq!(error_text(&answer).0, "invalid", "{answer}");
    let fields = answer["error"]["fields"].as_array().expect("fields");
    let named: Vec<&str> = fields
        .iter()
        .filter_map(|field| field["field"].as_str())
        .collect();
    for field in ["name", "text", "repeat"] {
        assert!(named.contains(&field), "{field}: {fields:?}");
    }
    let listed = world.client.ask("schedule.list", json!({})).await;
    assert_eq!(listed["result"]["schedules"], json!([]), "nothing was kept");
    world.stop().await;
}

#[tokio::test]
async fn ten_schedules_and_their_history_survive_a_restart_and_the_ones_that_are_on_run() {
    let time = FakeTime::leak(start_of_day());
    let mut world = World::open_with(FakeClis::new(), options(time), time).await;
    let agent = world.ready_agent("Ana").await;
    let mut ids = Vec::new();
    for number in 1..=10 {
        let on = number % 2 == 0;
        let answer = world
            .save(&format!("s{number}"), &agent, "x", every_hours(1), on)
            .await;
        ids.push(
            answer["result"]["schedule"]["id"]
                .as_str()
                .expect("id")
                .to_owned(),
        );
    }
    world
        .client
        .ask("schedule.run_now", json!({"schedule_id": ids[0]}))
        .await;
    world.wait_for_runs(&ids[0], 1).await;
    let mut before = Vec::new();
    for id in &ids {
        before.push((world.schedule(id).await, world.history(id).await));
    }

    let mut world = restart(world, time).await;

    for (id, (schedule, history)) in ids.iter().zip(&before) {
        assert_eq!(&world.schedule(id).await, schedule);
        assert_eq!(&world.history(id).await, history);
    }
    world.client.start(&agent).await;
    world
        .client
        .wait_for_status(&agent, "waiting_for_input")
        .await;
    time.advance(HOUR + SECOND);
    for id in ids.iter().skip(1).step_by(2) {
        world.wait_for_runs(id, 1).await;
    }
    world.stop().await;
}

#[tokio::test]
async fn an_open_run_at_a_server_start_is_failed_with_the_reason_that_the_server_stopped() {
    let time = FakeTime::leak(start_of_day());
    let mut world = World::open_with(FakeClis::new(), options(time), time).await;
    let agent = world.ready_agent("Ana").await;
    let id = world.saved("hourly", &agent, "x", every_hours(24)).await;
    let database = world.office.serving.office.config.database_path();
    let world_after = {
        let stopped = Box::pin(restart_without_start(world)).await;
        // The server is stopped; leave a run with no result, as a crash does.
        let connection = rusqlite::Connection::open(&database).expect("database");
        connection
            .execute(
                "INSERT INTO schedule_runs (schedule_id, due, started, via) VALUES (?1, 0, 0, 'due')",
                [&id],
            )
            .expect("an open run");
        drop(connection);
        stopped
    };

    let mut world = Box::pin(world_after.start_again(time)).await;

    let runs = world.history(&id).await;
    assert_eq!(runs.len(), 1, "{runs:?}");
    assert_eq!(runs[0]["result"], "failed");
    assert_eq!(runs[0]["reason"], "the server stopped");
    world.stop().await;
}

/// A stopped server and what a test needs to start it again.
struct StoppedWorld {
    stopped: support::running_office::RunningOffice,
    clis: FakeClis,
    work_folder: tempfile::TempDir,
    records: tempfile::TempDir,
}

async fn restart_without_start(world: World) -> StoppedWorld {
    let Office {
        serving,
        clis,
        work_folder,
        records,
    } = world.office;
    StoppedWorld {
        stopped: serving.stop().await,
        clis,
        work_folder,
        records,
    }
}

impl StoppedWorld {
    async fn start_again(self, time: &'static FakeTime) -> World {
        let serving = serve_again_with(self.stopped, self.clis.login_setup(), options(time)).await;
        let office = Office {
            serving,
            clis: self.clis,
            work_folder: self.work_folder,
            records: self.records,
        };
        let client = office.client().await;
        World {
            office,
            client,
            time,
        }
    }
}

/// The real server binary that the fake CLIs run as the helper.
const HELPER: &str = env!("CARGO_BIN_EXE_office-harnessd");

/// Places an agent through the same store the server built.
fn place(world: &World, agent_id: &str, role: Role, parent: Option<&str>) {
    let connection = rusqlite::Connection::open(world.office.serving.office.config.database_path())
        .expect("database");
    let store: Arc<dyn HierarchyStore> = Arc::new(HierarchyRepository::new(
        SharedConnection::new(connection).handle(),
        AgentPlaces::new(),
    ));
    PlaceAgentHandler::new(store)
        .handle(PlaceAgentRequest {
            agent_id: agent_id.parse().expect("agent id"),
            role,
            parent_agent_id: parent.map(|id| id.parse().expect("agent id")),
            team_id: None,
        })
        .expect("placement");
}

/// An agent that keeps running and hands its call credential to the records.
async fn calling_agent(world: &mut World, name: &str) -> (String, String, String) {
    let folder = world.records();
    let body = format!(
        "env > {0}/{name}.part\nmv {0}/{name}.part {0}/{name}.env\nsleep 30\n",
        folder.display()
    );
    let command = format!("cli-{name}");
    world.office.clis.add(&command, &body);
    let agent = world
        .client
        .create_agent(&world.office, name, &command, None)
        .await;
    world.client.start(&agent).await;
    let path = folder.join(format!("{name}.env"));
    let env = wait_for("the agent's environment", || fs::read_to_string(&path).ok()).await;
    let value = |variable: &str| {
        env.lines()
            .find_map(|line| line.strip_prefix(&format!("{variable}=")))
            .expect(variable)
            .to_owned()
    };
    (
        agent,
        value("OFFICE_HARNESS_AGENT_TOKEN"),
        value("OFFICE_HARNESS_SERVER_FILE"),
    )
}

/// Runs the helper as the agent, on a blocking thread: waiting for it on the
/// runtime thread would stop the server it talks to.
async fn agent_call(credential: &str, server_file: &str, kind: &str, fields: &Value) -> Value {
    let (credential, server_file) = (credential.to_owned(), server_file.to_owned());
    let (kind, fields) = (kind.to_owned(), fields.to_string());
    let output = tokio::task::spawn_blocking(move || {
        Command::new(HELPER)
            .args(["agent-call", &kind, &fields])
            .env("OFFICE_HARNESS_AGENT_TOKEN", credential)
            .env("OFFICE_HARNESS_SERVER_FILE", server_file)
            .stderr(Stdio::null())
            .output()
            .expect("helper")
    })
    .await
    .expect("helper task");
    serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim()).expect("json")
}

#[tokio::test]
async fn an_agent_that_asks_for_any_schedule_request_is_refused_and_recorded_and_nothing_changes() {
    let mut world = World::open().await;
    let (agent, credential, server_file) = calling_agent(&mut world, "Ana").await;
    let target = agent.clone();
    let mine = json!({
        "name": "mine", "text": "x", "target_agent_id": target,
        "repeat": every_hours(1), "on": true,
    });
    let someone = "00000000-0000-4000-8000-000000000001";
    let requests = [
        ("schedule.save", mine.clone()),
        (
            "schedule.save",
            json!({"schedule_id": someone, "name": "mine", "text": "x",
            "target_agent_id": target, "repeat": every_hours(1), "on": true}),
        ),
        (
            "schedule.switch",
            json!({"schedule_id": someone, "on": false}),
        ),
        ("schedule.run_now", json!({"schedule_id": someone})),
        ("schedule.delete", json!({"schedule_id": someone})),
    ];

    for (kind, fields) in &requests {
        let answer = agent_call(&credential, &server_file, kind, fields).await;
        assert_eq!(answer["ok"], false, "{kind}: {answer}");
        assert_eq!(answer["error"]["code"], "refused", "{kind}: {answer}");
        assert_eq!(
            answer["error"]["message"], "only the person makes schedules",
            "{kind}: {answer}"
        );
    }

    let entries = world.actions_of(&agent).await;
    let refused = entries
        .iter()
        .filter(|entry| entry["action"] == "refused" && entry["actor"] == format!("agent:{agent}"))
        .count();
    assert_eq!(refused, 5, "{entries:?}");
    let listed = world.client.ask("schedule.list", json!({})).await;
    assert_eq!(listed["result"]["schedules"], json!([]), "{listed}");
    world.stop().await;
}

#[tokio::test]
async fn a_hold_on_an_ancestor_of_a_target_that_was_moved_refuses_the_run_with_the_tree_as_it_is() {
    let mut world = World::open().await;
    let member = world.ready_agent("Member").await;
    let ceo = world
        .agent_running_working("Chief", "while true; do echo tick; sleep 0.1; done")
        .await;
    let id = world.saved("moved", &member, "once", every_hours(24)).await;
    world
        .client
        .ask("schedule.run_now", json!({"schedule_id": id}))
        .await;
    world.wait_for_result(&id, "submitted").await;

    // The member is taken under the CEO, and then the CEO is held.
    place(&world, &ceo, Role::Ceo, None);
    place(&world, &member, Role::Lead, Some(&ceo));
    world.client.ask("hold.set", json!({"agent_id": ceo})).await;
    world
        .client
        .ask("schedule.run_now", json!({"schedule_id": id}))
        .await;

    let refused = world.wait_for_result(&id, "refused").await;
    assert!(refused["reason"].as_str().expect("reason").contains("hold"));
    world.stop().await;
}

#[tokio::test]
async fn a_subtree_delete_switches_off_the_schedule_of_a_deleted_descendant() {
    let mut world = World::open().await;
    let ceo = world.ready_agent("Chief").await;
    let member = world.ready_agent("Member").await;
    place(&world, &ceo, Role::Ceo, None);
    place(&world, &member, Role::Lead, Some(&ceo));
    let id = world.saved("member", &member, "x", every_hours(1)).await;

    let deleted = world
        .client
        .ask(
            "agent.delete",
            json!({"agent_id": ceo, "include_descendants": true, "descendant_count": 1}),
        )
        .await;

    assert_eq!(deleted["ok"], true, "{deleted}");
    let after = world.schedule(&id).await;
    assert_eq!(after["on"], false, "{after}");
    assert_eq!(after["off_reason"], "target_deleted");
    assert_eq!(world.schedule_notifications().await.len(), 1);
    world.stop().await;
}

/// A repository with one commit, which a temp needs for its own worktree.
fn repository_with_a_commit(folder: &Path) {
    let git = |arguments: &[&str]| {
        let home = folder.parent().expect("parent").join("home");
        fs::create_dir_all(&home).expect("home");
        let output = Command::new("git")
            .args(["-c", "commit.gpgsign=false", "-c", "user.name=Test"])
            .args(["-c", "user.email=test@example.com", "-C"])
            .arg(folder)
            .args(arguments)
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", &home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .expect("git runs");
        assert!(output.status.success(), "git {arguments:?}");
    };
    fs::create_dir_all(folder).expect("repository folder");
    git(&["init", "--quiet", "--initial-branch=main"]);
    fs::write(folder.join("a.txt"), "a\n").expect("file");
    git(&["add", "--all"]);
    git(&["commit", "--quiet", "-m", "first"]);
}

#[tokio::test]
async fn the_end_of_a_temp_that_is_the_target_switches_the_schedule_off() {
    let clis = FakeClis::new();
    let environment = std::env::vars().collect();
    let git = find_program("git", &environment, Path::new("/")).expect("git is installed");
    clis.add("git", &format!("exec '{}' \"$@\"", git.display()));
    let time = FakeTime::leak(start_of_day());
    let mut world = World::open_with(clis, options(time), time).await;
    let repository_folder = tempfile::tempdir().expect("repository folder");
    let repository = repository_folder.path().join("repo");
    repository_with_a_commit(&repository);
    world.office.clis.add("cli-Temp", "echo ready; sleep 30");
    let created = world
        .client
        .ask(
            "agent.create",
            json!({
                "name": "Temp", "command": "cli-Temp", "working_folder": repository,
                "own_worktree": true, "temp": true, "objective": "short job",
            }),
        )
        .await;
    assert_eq!(created["ok"], true, "{created}");
    let temp = created["result"]["id"].as_str().expect("id").to_owned();
    let id = world.saved("temp job", &temp, "x", every_hours(1)).await;

    let ended = world
        .client
        .ask("temp.end", json!({"agent_id": temp}))
        .await;
    assert_eq!(ended["ok"], true, "{ended}");

    let deadline = Instant::now() + RESULT_LIMIT;
    while world.schedule(&id).await["off_reason"] != "target_deleted" {
        assert!(Instant::now() < deadline, "the schedule never went off");
        tokio::time::sleep(POLL).await;
    }
    assert_eq!(world.schedule(&id).await["on"], false);
    assert_eq!(world.schedule_notifications().await.len(), 1);
    world.stop().await;
}

#[tokio::test]
async fn fifty_schedules_due_in_the_same_minute_for_agents_that_wait_all_run() {
    let mut world = World::open().await;
    let mut ids = Vec::new();
    let mut names = Vec::new();
    for number in 0..12 {
        let name = format!("A{number}");
        let agent = world.ready_agent(&name).await;
        let id = world
            .saved(&format!("s{number}"), &agent, "go", every_hours(1))
            .await;
        ids.push(id);
        names.push(name);
    }

    world.time.advance(HOUR + SECOND);

    for id in &ids {
        world.wait_for_result(id, "submitted").await;
    }
    for name in &names {
        world.wait_for_typed(name, "go\r").await;
    }
    world.stop().await;
}
