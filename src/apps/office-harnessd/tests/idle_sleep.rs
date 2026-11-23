// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]
#![cfg(unix)]

//! 0018 at protocol level: the idle sleep setting, the sleep of an idle
//! agent, the blocking conditions, and `wake.start` with its `wake.result`
//! events, through a real socket, with fake CLIs and a short minute.

mod support;

use std::time::{Duration, Instant};

use office_harnessd::server::ServerOptions;
use serde_json::{Value, json};
use support::fake_cli::FakeClis;
use support::office_client::{Client, Office, error_text, recording_body, records_of};
use support::running_office::serve_again;

/// One minute of the setting; 5 minutes pass in half a second.
const MINUTE: Duration = Duration::from_millis(100);
const TICK: Duration = Duration::from_millis(50);
const IDLE_MINUTES: u32 = 5;
const CHECK_PERIOD: Duration = Duration::from_secs(2);
const RESULT_WAIT: Duration = Duration::from_secs(60);
/// Three idle times: long enough for any sleep the rule would allow.
const AWAKE_WATCH: Duration = Duration::from_millis(1_500);
const KEEPS_RUNNING: &str = "echo ready; sleep 30";
const ENDS_AT_ONCE: &str = "echo 'the session cannot be continued'; exit 1";
/// The first run of a fake CLI keeps running until the sleep; every later run
/// follows the test's behavior.
const FIRST_RUN_WAITS: &str =
    "if [ ! -e first-run-done ]; then : > first-run-done; echo ready; sleep 30; fi";

async fn open_office() -> Office {
    Office::open_with_options(
        FakeClis::new(),
        ServerOptions {
            resume_check_period: CHECK_PERIOD,
            idle_sleep_tick: TICK,
            idle_sleep_minute: MINUTE,
            ..ServerOptions::default()
        },
    )
    .await
}

/// A claude stand-in: `on_resume` runs for `--resume`, `on_new` for a new
/// session after the first run.
fn claude_body(office: &Office, on_resume: &str, on_new: &str) -> String {
    recording_body(
        office.records_path(),
        &format!(
            "cd {}\n{FIRST_RUN_WAITS}\ncase \"$1\" in --resume) {on_resume};; *) {on_new};; esac",
            office.records_path().display()
        ),
    )
}

async fn turn_on(client: &mut Client, minutes: u32) {
    client
        .set_setting("idle_sleep_minutes", &minutes.to_string())
        .await;
}

/// Starts the agent and waits until it is idle and its Needs you item, which
/// every new `waiting for input` brings, is dismissed.
async fn idle_agent(client: &mut Client, office: &Office, name: &str, command: &str) -> String {
    let agent = client.create_agent(office, name, command, None).await;
    client.start(&agent).await;
    client.wait_for_status(&agent, "waiting_for_input").await;
    client.dismiss_needs_you(&agent).await;
    agent
}

async fn actions_of(client: &mut Client, agent: &str) -> Vec<Value> {
    let answer = client
        .ask("agent.actions", json!({"agent_id": agent}))
        .await;
    answer["result"].as_array().expect("entries").clone()
}

fn entries<'a>(actions: &'a [Value], action: &str) -> Vec<&'a Value> {
    actions
        .iter()
        .filter(|entry| entry["action"] == action)
        .collect()
}

/// Asserts that the agent stays awake for three idle times.
async fn stays_awake(client: &mut Client, agent: &str) {
    tokio::time::sleep(AWAKE_WATCH).await;
    assert_eq!(client.status_of(agent).await, "waiting_for_input");
    assert_eq!(entries(&actions_of(client, agent).await, "sleep").len(), 0);
}

/// The `wake.result` events of one agent, up to and with its final one.
async fn wake_results_of(client: &mut Client, agent: &str) -> Vec<Value> {
    let mut results = Vec::new();
    loop {
        let event = tokio::time::timeout(RESULT_WAIT, client.next_event("wake.result"))
            .await
            .expect("a wake result");
        if event["agent_id"] != agent {
            continue;
        }
        let is_final = event["is_final"] == true;
        results.push(event);
        if is_final {
            return results;
        }
    }
}

fn result_names(results: &[Value]) -> Vec<&str> {
    results
        .iter()
        .filter_map(|result| result["result"].as_str())
        .collect()
}

#[tokio::test]
async fn with_idle_sleep_off_no_idle_agent_is_put_to_sleep() {
    let office = open_office().await;
    office.clis.add("plain-cli", KEEPS_RUNNING);
    let mut client = office.client().await;
    let agent = idle_agent(&mut client, &office, "Ana", "plain-cli").await;

    stays_awake(&mut client, &agent).await;
    office.serving.stop().await;
}

#[tokio::test]
async fn the_setting_takes_0_and_5_to_1440_minutes_only() {
    let office = open_office().await;
    let mut client = office.client().await;
    for refused in ["4", "1441", "2.5", "soon"] {
        let answer = client
            .ask(
                "settings.set",
                json!({"key": "idle_sleep_minutes", "value": refused}),
            )
            .await;
        assert_eq!(answer["ok"], false, "{refused}: {answer}");
        assert_eq!(
            error_text(&answer),
            (
                "invalid",
                "idle_sleep_minutes is 0 (off) or a whole number of minutes from 5 to 1440"
            )
        );
    }
    let stored = client
        .ask("settings.get", json!({"key": "idle_sleep_minutes"}))
        .await;
    assert_eq!(stored["result"]["value"], "0", "{stored}");
    office.serving.stop().await;
}

#[tokio::test]
async fn an_idle_agent_nobody_needs_is_put_to_sleep_with_one_record_and_no_notification() {
    let office = open_office().await;
    office.clis.add("plain-cli", KEEPS_RUNNING);
    let mut client = office.client().await;
    let agent = idle_agent(&mut client, &office, "Ana", "plain-cli").await;
    let notifications_before = client.ask("notification.list", json!({})).await;

    turn_on(&mut client, IDLE_MINUTES).await;
    client.wait_for_status(&agent, "asleep").await;

    let actions = actions_of(&mut client, &agent).await;
    let sleeps = entries(&actions, "sleep");
    assert_eq!(sleeps.len(), 1, "{actions:?}");
    assert_eq!(sleeps[0]["actor"], "office");
    assert_eq!(sleeps[0]["detail"], "idle for 5 minutes");
    assert!(entries(&actions, "stop").is_empty(), "{actions:?}");
    let notifications_after = client.ask("notification.list", json!({})).await;
    assert_eq!(
        notifications_after["result"],
        notifications_before["result"]
    );
    let needs_you = client.ask("needs_you.list", json!({})).await;
    assert_eq!(needs_you["result"]["items"], json!([]), "{needs_you}");
    let sessions = client.sessions(&agent).await;
    assert!(!sessions[0]["ended_at"].is_null(), "{sessions:?}");
    office.serving.stop().await;
}

#[tokio::test]
async fn turning_idle_sleep_on_restarts_the_count_for_long_idle_agents() {
    let office = open_office().await;
    office.clis.add("plain-cli", KEEPS_RUNNING);
    let mut client = office.client().await;
    let agent = idle_agent(&mut client, &office, "Ana", "plain-cli").await;
    // Idle for three idle times before the person turns the setting on.
    tokio::time::sleep(AWAKE_WATCH).await;

    turn_on(&mut client, IDLE_MINUTES).await;
    let turned_on = Instant::now();
    tokio::time::sleep(MINUTE * IDLE_MINUTES / 2).await;
    assert_ne!(
        client.status_of(&agent).await,
        "asleep",
        "asleep {} ms after the setting was turned on",
        turned_on.elapsed().as_millis()
    );
    client.wait_for_status(&agent, "asleep").await;
    office.serving.stop().await;
}

#[tokio::test]
async fn an_item_in_needs_you_keeps_the_agent_awake() {
    let office = open_office().await;
    office.clis.add("plain-cli", KEEPS_RUNNING);
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "plain-cli", None).await;
    client.start(&agent).await;
    client.wait_for_status(&agent, "waiting_for_input").await;

    turn_on(&mut client, IDLE_MINUTES).await;
    stays_awake(&mut client, &agent).await;
    office.serving.stop().await;
}

#[tokio::test]
async fn a_hold_keeps_the_agent_awake() {
    let office = open_office().await;
    office.clis.add("plain-cli", KEEPS_RUNNING);
    let mut client = office.client().await;
    let agent = idle_agent(&mut client, &office, "Ana", "plain-cli").await;
    let held = client.ask("hold.set", json!({"agent_id": agent})).await;
    assert_eq!(held["ok"], true, "{held}");

    turn_on(&mut client, IDLE_MINUTES).await;
    stays_awake(&mut client, &agent).await;
    office.serving.stop().await;
}

#[tokio::test]
async fn unsent_person_text_keeps_the_agent_awake() {
    let office = open_office().await;
    office.clis.add("plain-cli", KEEPS_RUNNING);
    let mut client = office.client().await;
    let agent = idle_agent(&mut client, &office, "Ana", "plain-cli").await;
    // "hi" without Enter (0012 A5); the CLI does not echo it.
    let typed = client
        .ask("terminal.input", json!({"agent_id": agent, "data": "aGk="}))
        .await;
    assert_eq!(typed["ok"], true, "{typed}");

    turn_on(&mut client, IDLE_MINUTES).await;
    stays_awake(&mut client, &agent).await;
    office.serving.stop().await;
}

#[tokio::test]
async fn a_client_cannot_ask_for_a_sleep() {
    let office = open_office().await;
    office.clis.add("plain-cli", KEEPS_RUNNING);
    let mut client = office.client().await;
    let agent = idle_agent(&mut client, &office, "Ana", "plain-cli").await;

    let answer = client
        .ask(
            "agent.sleep",
            json!({"agent_id": agent, "idle_minutes": IDLE_MINUTES}),
        )
        .await;

    assert_eq!(
        error_text(&answer),
        ("invalid", "agent.sleep is made by the office only")
    );
    assert_eq!(client.status_of(&agent).await, "waiting_for_input");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_client_cannot_wake_an_agent_past_the_watch() {
    let office = open_office().await;
    office.clis.add("plain-cli", KEEPS_RUNNING);
    let mut client = office.client().await;
    let agent = idle_agent(&mut client, &office, "Ana", "plain-cli").await;
    turn_on(&mut client, IDLE_MINUTES).await;
    client.wait_for_status(&agent, "asleep").await;
    let sessions_before = client.sessions(&agent).await.len();

    let answer = client.ask("agent.wake", json!({"agent_id": agent})).await;

    assert_eq!(
        error_text(&answer),
        ("invalid", "agent.wake is made by wake.start only")
    );
    assert_eq!(client.status_of(&agent).await, "asleep");
    assert_eq!(client.sessions(&agent).await.len(), sessions_before);
    office.serving.stop().await;
}

#[tokio::test]
async fn a_wake_continues_the_last_session_and_records_one_wake_by_the_person() {
    let office = open_office().await;
    office.clis.add(
        "claude",
        &claude_body(&office, KEEPS_RUNNING, KEEPS_RUNNING),
    );
    let mut client = office.client().await;
    let agent = idle_agent(&mut client, &office, "Ana", "claude").await;
    turn_on(&mut client, IDLE_MINUTES).await;
    client.wait_for_status(&agent, "asleep").await;
    // A woken agent must not fall asleep again during the checks.
    turn_on(&mut client, 1440).await;

    let request = client.send("wake.start", json!({"agent_id": agent})).await;
    let results = wake_results_of(&mut client, &agent).await;
    let answer = client.answer_of(request).await;

    assert_eq!(answer["ok"], true, "{answer}");
    assert_eq!(answer["result"]["continued"], true, "{answer}");
    assert_eq!(result_names(&results), ["checking", "continued"]);
    let actions = actions_of(&mut client, &agent).await;
    let wakes = entries(&actions, "wake");
    assert_eq!(wakes.len(), 1, "{actions:?}");
    assert_eq!(wakes[0]["actor"], "user");
    assert!(wakes[0]["detail"].is_null(), "{wakes:?}");
    let records = records_of(office.records_path());
    assert!(
        records.last().is_some_and(|record| record
            .arguments
            .first()
            .is_some_and(|arg| arg == "--resume")),
        "the last run continued the session"
    );
    office.serving.stop().await;
}

#[tokio::test]
async fn a_wake_under_a_hold_continues_the_session_and_the_hold_stays() {
    let office = open_office().await;
    office.clis.add(
        "claude",
        &claude_body(&office, KEEPS_RUNNING, KEEPS_RUNNING),
    );
    let mut client = office.client().await;
    let agent = idle_agent(&mut client, &office, "Ana", "claude").await;
    turn_on(&mut client, IDLE_MINUTES).await;
    client.wait_for_status(&agent, "asleep").await;
    turn_on(&mut client, 1440).await;
    let held = client.ask("hold.set", json!({"agent_id": agent})).await;
    assert_eq!(held["ok"], true, "{held}");

    let request = client.send("wake.start", json!({"agent_id": agent})).await;
    let results = wake_results_of(&mut client, &agent).await;
    let answer = client.answer_of(request).await;

    assert_eq!(answer["result"]["continued"], true, "{answer}");
    assert_eq!(result_names(&results), ["checking", "continued"]);
    let holds = client.ask("hold.list", json!({})).await;
    let agent_holds = holds["result"]["agents"].as_array().expect("agent holds");
    assert!(
        agent_holds.iter().any(|hold| hold["agent_id"] == agent),
        "{holds}"
    );
    office.serving.stop().await;
}

#[tokio::test]
async fn a_wake_that_fails_twice_starts_the_agent_twice_and_holds_it_with_one_notification() {
    let office = open_office().await;
    office
        .clis
        .add("claude", &claude_body(&office, ENDS_AT_ONCE, ENDS_AT_ONCE));
    let mut client = office.client().await;
    let agent = idle_agent(&mut client, &office, "Ana", "claude").await;
    turn_on(&mut client, IDLE_MINUTES).await;
    client.wait_for_status(&agent, "asleep").await;

    let request = client.send("wake.start", json!({"agent_id": agent})).await;
    let results = wake_results_of(&mut client, &agent).await;
    client.answer_of(request).await;

    assert_eq!(result_names(&results), ["checking", "checking", "held"]);
    let reason = results[2]["reason"].as_str().expect("reason");
    assert!(reason.starts_with("the wake failed"), "{reason}");
    let notifications = client.ask("notification.list", json!({})).await;
    let holds = notifications["result"]
        .as_array()
        .expect("notifications")
        .iter()
        .filter(|row| row["agent_id"] == agent.as_str() && row["event"] == "resume_hold")
        .count();
    assert_eq!(holds, 1, "{notifications}");
    assert_eq!(
        records_of(office.records_path()).len(),
        3,
        "the first run, the continue, and one new session"
    );
    office.serving.stop().await;
}

#[tokio::test]
async fn a_wake_of_an_agent_that_is_not_asleep_starts_nothing() {
    let office = open_office().await;
    office.clis.add("plain-cli", KEEPS_RUNNING);
    let mut client = office.client().await;
    let agent = idle_agent(&mut client, &office, "Ana", "plain-cli").await;

    let answer = client.ask("wake.start", json!({"agent_id": agent})).await;

    assert_eq!(
        error_text(&answer),
        ("conflict", "the agent is no longer asleep")
    );
    office.serving.stop().await;
}

#[tokio::test]
async fn two_wakes_at_once_start_the_agent_once() {
    let office = open_office().await;
    office.clis.add(
        "claude",
        &claude_body(&office, KEEPS_RUNNING, KEEPS_RUNNING),
    );
    let mut client = office.client().await;
    let agent = idle_agent(&mut client, &office, "Ana", "claude").await;
    turn_on(&mut client, IDLE_MINUTES).await;
    client.wait_for_status(&agent, "asleep").await;
    turn_on(&mut client, 1440).await;

    let first = client.send("wake.start", json!({"agent_id": agent})).await;
    let second = client.send("wake.start", json!({"agent_id": agent})).await;
    let answers = [
        client.answer_of(first).await,
        client.answer_of(second).await,
    ];

    let woken = answers.iter().filter(|answer| answer["ok"] == true).count();
    assert_eq!(woken, 1, "{answers:?}");
    let refused = answers
        .iter()
        .find(|answer| answer["ok"] == false)
        .expect("one refusal");
    assert_eq!(
        error_text(refused),
        ("conflict", "the agent is no longer asleep")
    );
    office.serving.stop().await;
}

#[tokio::test]
async fn an_asleep_agent_stays_asleep_after_a_server_restart_and_is_not_resumable() {
    let mut office = open_office().await;
    office.clis.add("plain-cli", KEEPS_RUNNING);
    let mut client = office.client().await;
    let agent = idle_agent(&mut client, &office, "Ana", "plain-cli").await;
    turn_on(&mut client, IDLE_MINUTES).await;
    client.wait_for_status(&agent, "asleep").await;
    drop(client);

    let stopped = office.serving.stop().await;
    office.serving = serve_again(stopped, office.clis.login_setup()).await;
    let mut client = office.client().await;

    assert_eq!(client.status_of(&agent).await, "asleep");
    let plan = client.ask("resume.plan", json!({})).await;
    assert_eq!(plan["result"]["agents"], json!([]), "{plan}");
    assert_eq!(client.sessions(&agent).await.len(), 1, "nothing started");
    let setting = client
        .ask("settings.get", json!({"key": "idle_sleep_minutes"}))
        .await;
    assert_eq!(
        setting["result"]["value"],
        IDLE_MINUTES.to_string(),
        "{setting}"
    );
    office.serving.stop().await;
}

#[tokio::test]
async fn a_person_stop_during_a_batch_of_slow_sleeps_waits_at_most_one_grace_time() {
    const GRACE: Duration = Duration::from_secs(1);
    const MARGIN: Duration = Duration::from_secs(2);
    let office = open_office().await;
    office
        .clis
        .add("slow-cli", "trap '' INT TERM HUP; echo ready; sleep 30");
    office.clis.add("plain-cli", KEEPS_RUNNING);
    let mut client = office.client().await;
    client
        .set_setting("stop_grace_ms", &GRACE.as_millis().to_string())
        .await;
    let mut slow = Vec::new();
    for name in ["Ana", "Ben", "Cem"] {
        slow.push(idle_agent(&mut client, &office, name, "slow-cli").await);
    }
    let other = client
        .create_agent(&office, "Dora", "plain-cli", None)
        .await;
    client.start(&other).await;
    client.wait_for_status(&other, "working").await;

    turn_on(&mut client, IDLE_MINUTES).await;
    client.wait_for_status(&slow[0], "asleep").await;
    let asked = Instant::now();
    let stopped = client.ask("agent.stop", json!({"agent_id": other})).await;

    assert_eq!(stopped["ok"], true, "{stopped}");
    assert!(
        asked.elapsed() <= GRACE + MARGIN,
        "the stop took {} ms",
        asked.elapsed().as_millis()
    );
    office.serving.stop().await;
}
