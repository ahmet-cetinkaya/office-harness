// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]
#![cfg(unix)]

//! 0019 at protocol level: the sign-in status command before a start and
//! the watch of the first seconds, with fake CLIs named `claude` and
//! `gemini` behind a real socket.

mod support;

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use support::fake_cli::FakeClis;
use support::office_client::{Client, Office};

const SECRET_LINE: &str = "sk-test-never-shown";
/// A status command that says the CLI is not signed in, with a line shaped
/// like a key that must never reach the person.
const NOT_SIGNED_IN: &str = "echo '{\"token\": \"sk-test-never-shown\"}'; printf 'Not logged in.\\033[2J\\342\\200\\256 Run claude auth login.\\n' >&2; exit 1";
const SIGNED_IN: &str = "echo 'Logged in. sk-test-never-shown'; exit 0";
const KEEPS_RUNNING: &str = "echo running; sleep 30";
const SIGN_IN_SCREEN: &str = "How would you like to authenticate for this project?";
const ENDED_WITHIN: Duration = Duration::from_secs(6);
const POLL: Duration = Duration::from_millis(100);

/// A `claude` stand-in: `auth status` counts the run, keeps the names of
/// its environment, and runs `on_status`; every other call is a session.
fn add_claude(office: &Office, on_status: &str) {
    let records = office.records_path().display();
    office.clis.add_with_status_answer(
        "claude",
        &format!(
            "if [ \"$1 $2\" = 'auth status' ]; then echo run >> {records}/status-runs; env | cut -d= -f1 > {records}/status-env; {on_status}; fi"
        ),
        KEEPS_RUNNING,
    );
}

fn status_runs(office: &Office) -> usize {
    fs::read_to_string(office.records_path().join("status-runs"))
        .map_or(0, |runs| runs.lines().count())
}

async fn actions_of(client: &mut Client, agent: &str) -> Vec<Value> {
    let answer = client
        .ask("agent.actions", json!({"agent_id": agent}))
        .await;
    answer["result"].as_array().expect("entries").clone()
}

fn start_entry(actions: &[Value]) -> &Value {
    actions
        .iter()
        .find(|entry| entry["action"] == "start")
        .expect("start entry")
}

async fn wait_for_end(client: &mut Client, agent: &str, limit: Duration) -> String {
    let deadline = Instant::now() + limit;
    loop {
        let status = client.status_of(agent).await;
        if ["failed", "exited", "stopped"].contains(&status.as_str()) {
            return status;
        }
        assert!(Instant::now() < deadline, "{agent} still {status}");
        tokio::time::sleep(POLL).await;
    }
}

fn assert_no_secret(texts: &[&Value]) {
    for text in texts {
        assert!(!text.to_string().contains(SECRET_LINE), "{text}");
    }
}

#[tokio::test]
async fn a_start_of_a_cli_that_is_not_signed_in_fails_with_one_visible_reason() {
    let office = Office::open().await;
    add_claude(&office, NOT_SIGNED_IN);
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "claude", None).await;

    let started = client.start(&agent).await;

    assert_eq!(client.status_of(&agent).await, "failed");
    let sessions = client.sessions(&agent).await;
    let ended = sessions[0]["exit_result"].as_str().expect("exit result");
    assert_eq!(
        ended,
        "spawn-error: claude is not signed in: `claude auth status --text` said so. Sign in to \
         claude in your own terminal, then start the agent again. It showed: Not \
         logged in.␛[2J\\u202E Run claude auth login."
    );
    let actions = actions_of(&mut client, &agent).await;
    let start = start_entry(&actions);
    assert_eq!(start["actor"], "user");
    assert!(
        start["result"]
            .as_str()
            .is_some_and(|result| result.starts_with("claude is not signed in")),
        "{start}"
    );
    assert_no_secret(&[&started, &Value::from(sessions), &Value::from(actions)]);
    let log = fs::read_to_string(office.serving.office.config.log_path()).unwrap_or_default();
    assert!(!log.contains(SECRET_LINE));
    // One run: the check before the status order; none in place under it.
    assert_eq!(status_runs(&office), 1);
    let names = fs::read_to_string(office.records_path().join("status-env")).expect("env names");
    assert!(
        !names
            .lines()
            .any(|name| name.starts_with("OFFICE_HARNESS_")),
        "{names}"
    );
}

#[tokio::test]
async fn a_signed_in_cli_starts_and_its_status_output_shows_nowhere() {
    let office = Office::open().await;
    add_claude(&office, SIGNED_IN);
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "claude", None).await;

    let started = client.start(&agent).await;

    assert_ne!(client.status_of(&agent).await, "failed");
    assert_eq!(status_runs(&office), 1);
    let actions = actions_of(&mut client, &agent).await;
    assert_eq!(start_entry(&actions)["detail"], Value::Null, "{actions:?}");
    assert_no_secret(&[&started, &Value::from(actions)]);
}

#[tokio::test]
async fn a_hanging_status_command_blocks_no_stop_and_the_start_goes_on() {
    let office = Office::open().await;
    let records = office.records_path().display().to_string();
    add_claude(
        &office,
        &format!("sleep 30 & echo $! > {records}/status-child; wait"),
    );
    office.clis.add("worker", KEEPS_RUNNING);
    let mut client = office.client().await;
    let mut other_client = office.client().await;
    let worker = client.create_agent(&office, "Bo", "worker", None).await;
    client.start(&worker).await;
    let agent = client.create_agent(&office, "Ana", "claude", None).await;

    let began = Instant::now();
    let start = client.send("agent.start", json!({"agent_id": agent})).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let stop_began = Instant::now();
    let stopped = other_client
        .ask("agent.stop", json!({"agent_id": worker}))
        .await;
    assert_eq!(stopped["ok"], true, "{stopped}");
    assert!(
        stop_began.elapsed() < Duration::from_secs(1),
        "{:?}",
        stop_began.elapsed()
    );
    let answer = client.answer_of(start).await;

    assert_eq!(answer["ok"], true, "{answer}");
    assert!(began.elapsed() < ENDED_WITHIN, "{:?}", began.elapsed());
    assert_ne!(client.status_of(&agent).await, "failed");
    let actions = actions_of(&mut client, &agent).await;
    assert_eq!(
        start_entry(&actions)["detail"],
        "the sign-in check gave no answer: it timed out after 5 s"
    );
    let child: i32 = fs::read_to_string(Path::new(&records).join("status-child"))
        .expect("pid")
        .trim()
        .parse()
        .expect("number");
    tokio::time::sleep(Duration::from_secs(1)).await;
    let still_runs = std::process::Command::new("kill")
        .args(["-0", &child.to_string()])
        .status()
        .is_ok_and(|status| status.success());
    assert!(!still_runs, "the status command's child {child} still runs");
}

#[tokio::test]
async fn own_arguments_or_a_key_variable_skip_the_check() {
    let office = Office::open().await;
    add_claude(&office, NOT_SIGNED_IN);
    let mut client = office.client().await;
    let with_arguments = client
        .create_agent(&office, "Ana", "claude --settings mine.json", None)
        .await;
    let with_key = client
        .create_agent(
            &office,
            "Bo",
            "claude",
            Some(json!({"env": [{"name": "ANTHROPIC_API_KEY", "value": "set"}]})),
        )
        .await;

    client.start(&with_arguments).await;
    client.start(&with_key).await;

    assert_eq!(status_runs(&office), 0);
    assert_ne!(client.status_of(&with_arguments).await, "failed");
    assert_ne!(client.status_of(&with_key).await, "failed");
}

#[tokio::test]
async fn a_sign_in_screen_in_the_first_seconds_ends_the_cli_as_not_signed_in() {
    let office = Office::open().await;
    office
        .clis
        .add("gemini", &format!("echo '{SIGN_IN_SCREEN}'; sleep 30"));
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "gemini", None).await;

    client.start(&agent).await;

    assert_eq!(
        wait_for_end(&mut client, &agent, ENDED_WITHIN).await,
        "failed"
    );
    let sessions = client.sessions(&agent).await;
    let ended = sessions[0]["exit_result"].as_str().expect("exit result");
    assert!(
        ended.starts_with(
            "not-signed-in: gemini is not signed in: its first screen asks to sign in."
        ),
        "{ended}"
    );
    let actions = actions_of(&mut client, &agent).await;
    let stops: Vec<&Value> = actions
        .iter()
        .filter(|entry| entry["action"] == "stop" && entry["actor"] == "office")
        .collect();
    assert_eq!(stops.len(), 1, "{actions:?}");
    let notifications = client.ask("notification.list", json!({})).await;
    let failures = notifications["result"]
        .as_array()
        .expect("notifications")
        .iter()
        .filter(|row| row["agent_id"] == agent.as_str() && row["event"] == "failed")
        .count();
    assert_eq!(failures, 1, "{notifications}");
}

#[tokio::test]
async fn a_key_typed_into_an_agent_stops_only_its_own_watch() {
    let office = Office::open().await;
    office.clis.add(
        "gemini",
        &format!("echo ready; sleep 2; echo '{SIGN_IN_SCREEN}'; sleep 30"),
    );
    let mut client = office.client().await;
    let typed_into = client.create_agent(&office, "Ana", "gemini", None).await;
    let untouched = client.create_agent(&office, "Bo", "gemini", None).await;
    client.start(&typed_into).await;
    client.start(&untouched).await;

    let typed = client
        .ask(
            "terminal.input",
            json!({"agent_id": typed_into, "data": "aGkK"}),
        )
        .await;
    assert_eq!(typed["ok"], true, "{typed}");

    assert_eq!(
        wait_for_end(&mut client, &untouched, ENDED_WITHIN).await,
        "failed"
    );
    assert_ne!(client.status_of(&typed_into).await, "failed");
}

#[tokio::test]
async fn sign_in_words_after_the_watch_period_end_nothing() {
    let office =
        Office::open_with(FakeClis::new().with_sign_in_watch_period(Duration::from_secs(1))).await;
    office.clis.add(
        "gemini",
        &format!("echo ready; sleep 2; echo '{SIGN_IN_SCREEN}'; sleep 30"),
    );
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "gemini", None).await;

    client.start(&agent).await;
    tokio::time::sleep(Duration::from_secs(4)).await;

    assert_ne!(client.status_of(&agent).await, "failed");
}

#[tokio::test]
async fn a_startup_prompt_with_the_words_turns_the_watch_off() {
    let office = Office::open().await;
    office.clis.add("gemini", "echo ready; sleep 30");
    let mut client = office.client().await;
    let agent = client
        .create_agent(
            &office,
            "Ana",
            "gemini",
            Some(json!({"startup_prompt": "Answer: how would you like to authenticate?"})),
        )
        .await;

    client.start(&agent).await;
    tokio::time::sleep(Duration::from_secs(3)).await;

    assert_ne!(client.status_of(&agent).await, "failed");
}
