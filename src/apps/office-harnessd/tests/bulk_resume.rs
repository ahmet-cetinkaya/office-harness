// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]
#![cfg(unix)]

//! 0013 at protocol level: `resume.plan`, `resume.start`, and the
//! `resume.result` events, through a real socket, with fake CLIs and a short
//! check period.

mod support;

use std::time::Duration;

use agent_domain::enums::AgentStatus;
use agent_infrastructure::persistence::{AgentRepository, SharedConnection};
use office_harnessd::server::ServerOptions;
use rusqlite::Connection;
use serde_json::{Value, json};
use support::fake_cli::FakeClis;
use support::office_client::{Client, Office, error_text, recording_body};

const CHECK_PERIOD: Duration = Duration::from_secs(2);
const RESULT_WAIT: Duration = Duration::from_secs(60);
/// The first run of a fake CLI ends at once, so the test can mark the agent
/// interrupted; every later run follows the test's behavior.
const FIRST_RUN_ENDS: &str =
    "if [ ! -e first-run-done ]; then : > first-run-done; echo ready; exit 0; fi";
const RUNNING_STATUSES: [&str; 3] = ["starting", "working", "waiting_for_input"];
const KEEPS_RUNNING: &str = "echo running; sleep 30";
const ENDS_AT_ONCE: &str = "echo 'the session cannot be continued'; exit 1";

async fn open_office() -> Office {
    Office::open_with_options(
        FakeClis::new(),
        ServerOptions {
            resume_check_period: CHECK_PERIOD,
            ..ServerOptions::default()
        },
    )
    .await
}

/// A claude stand-in: `on_resume` runs for `--resume`, `on_new` for a new
/// session.
fn claude_body(office: &Office, on_resume: &str, on_new: &str) -> String {
    recording_body(
        office.records_path(),
        &format!(
            "cd {}\n{FIRST_RUN_ENDS}\ncase \"$1\" in --resume) {on_resume};; *) {on_new};; esac",
            office.records_path().display()
        ),
    )
}

/// Starts the agent once, waits for its first run to end, and marks it
/// interrupted as a server stop does.
async fn interrupted_agent(
    client: &mut Client,
    office: &Office,
    name: &str,
    command: &str,
) -> String {
    let agent = client.create_agent(office, name, command, None).await;
    client.start(&agent).await;
    client.wait_for_status(&agent, "exited").await;
    mark_interrupted(office, &agent);
    agent
}

fn mark_interrupted(office: &Office, agent: &str) {
    let connection =
        Connection::open(office.serving.office.config.database_path()).expect("database");
    AgentRepository::new(SharedConnection::new(connection))
        .update_status(agent.parse().expect("agent id"), AgentStatus::Interrupted)
        .expect("interrupted");
}

/// Sends `resume.start` without waiting: its results can arrive before its
/// answer, and `ask` would skip them.
async fn send_resume(client: &mut Client, agents: &[&str]) -> u32 {
    client
        .send("resume.start", json!({ "agent_ids": agents }))
        .await
}

async fn answered_ok(client: &mut Client, request: u32) -> Value {
    let answer = client.answer_of(request).await;
    assert_eq!(answer["ok"], true, "{answer}");
    answer["result"].clone()
}

/// The next `resume.result` event of one agent.
async fn next_result_of(client: &mut Client, agent: &str) -> Value {
    loop {
        let event = tokio::time::timeout(RESULT_WAIT, client.next_event("resume.result"))
            .await
            .expect("a resume result");
        if event["agent_id"] == agent {
            return event;
        }
    }
}

/// The `resume.result` events of one agent, up to and with its final one.
async fn results_of(client: &mut Client, agent: &str) -> Vec<Value> {
    let mut results = Vec::new();
    loop {
        let event = next_result_of(client, agent).await;
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

async fn actions_of(client: &mut Client, agent: &str) -> Vec<Value> {
    let answer = client
        .ask("agent.actions", json!({"agent_id": agent}))
        .await;
    answer["result"].as_array().expect("entries").clone()
}

fn office_starts(actions: &[Value]) -> Vec<&Value> {
    actions
        .iter()
        .filter(|entry| entry["action"] == "start" && entry["actor"] == "office")
        .collect()
}

#[tokio::test]
async fn an_interrupted_agent_continues_its_last_session_and_the_result_is_continued() {
    let office = open_office().await;
    office.clis.add(
        "claude",
        &claude_body(&office, KEEPS_RUNNING, KEEPS_RUNNING),
    );
    let mut client = office.client().await;
    let agent = interrupted_agent(&mut client, &office, "Ana", "claude").await;

    let request = send_resume(&mut client, &[&agent]).await;
    let results = results_of(&mut client, &agent).await;
    answered_ok(&mut client, request).await;

    assert_eq!(result_names(&results), ["checking", "continued"]);
    assert_eq!(results[0]["continued"], true);
    let kinds: Vec<Value> = actions_of(&mut client, &agent)
        .await
        .iter()
        .map(|entry| entry["action"].clone())
        .collect();
    assert!(kinds.contains(&json!("continue")), "{kinds:?}");
    let actions = actions_of(&mut client, &agent).await;
    assert_eq!(office_starts(&actions), Vec::<&Value>::new(), "{actions:?}");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_refused_continue_gets_one_new_session_recorded_with_the_office_and_its_reason() {
    let office = open_office().await;
    office
        .clis
        .add("claude", &claude_body(&office, ENDS_AT_ONCE, KEEPS_RUNNING));
    let mut client = office.client().await;
    let agent = interrupted_agent(&mut client, &office, "Ana", "claude").await;

    let request = send_resume(&mut client, &[&agent]).await;
    let results = results_of(&mut client, &agent).await;
    answered_ok(&mut client, request).await;

    assert_eq!(
        result_names(&results),
        ["checking", "checking", "new_session"]
    );
    assert_eq!(results[1]["continued"], false);
    let reason = results[2]["reason"].as_str().expect("reason");
    assert!(reason.contains("the CLI ended"), "{reason}");
    let actions = actions_of(&mut client, &agent).await;
    let starts = office_starts(&actions);
    assert_eq!(starts.len(), 1, "{actions:?}");
    assert!(
        starts[0]["detail"]
            .as_str()
            .is_some_and(|detail| detail.contains("the CLI ended")),
        "{starts:?}"
    );
    let status = client.status_of(&agent).await;
    assert!(RUNNING_STATUSES.contains(&status.as_str()), "{status}");
    office.serving.stop().await;
}

#[tokio::test]
async fn an_agent_that_fails_twice_is_started_twice_held_and_notified_once() {
    let office = open_office().await;
    office
        .clis
        .add("claude", &claude_body(&office, ENDS_AT_ONCE, ENDS_AT_ONCE));
    let mut client = office.client().await;
    let agent = interrupted_agent(&mut client, &office, "Ana", "claude").await;

    let request = send_resume(&mut client, &[&agent]).await;
    let results = results_of(&mut client, &agent).await;
    answered_ok(&mut client, request).await;

    assert_eq!(result_names(&results), ["checking", "checking", "held"]);
    let holds = client.ask("hold.list", json!({})).await;
    let held: Vec<&Value> = holds["result"]["agents"]
        .as_array()
        .expect("holds")
        .iter()
        .filter(|hold| hold["agent_id"] == agent.as_str())
        .collect();
    assert_eq!(held.len(), 1, "{holds}");
    assert_eq!(held[0]["set_by"], "office");
    assert!(
        held[0]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("resume failed"))
    );
    let notifications = client.ask("notification.list", json!({})).await;
    let resume_holds = notifications["result"]
        .as_array()
        .expect("notifications")
        .iter()
        .filter(|row| row["agent_id"] == agent.as_str() && row["event"] == "resume_hold")
        .count();
    assert_eq!(resume_holds, 1, "{notifications}");
    let records = support::office_client::records_of(office.records_path());
    assert_eq!(
        records.len(),
        3,
        "the first run, the continue, and one new session"
    );
    office.serving.stop().await;
}

#[tokio::test]
async fn a_cli_that_cannot_continue_starts_a_new_session_at_once_with_the_reason() {
    let office = open_office().await;
    office.clis.add(
        "plain-cli",
        &format!(
            "cd {}\n{FIRST_RUN_ENDS}\n{KEEPS_RUNNING}",
            office.records_path().display()
        ),
    );
    let mut client = office.client().await;
    let agent = interrupted_agent(&mut client, &office, "Ana", "plain-cli").await;
    let plan = client.ask("resume.plan", json!({})).await;

    let request = send_resume(&mut client, &[&agent]).await;
    let results = results_of(&mut client, &agent).await;
    answered_ok(&mut client, request).await;

    let entry = &plan["result"]["agents"][0];
    assert_eq!(entry["agent_id"], agent.as_str());
    assert_eq!(entry["can_continue"], false);
    assert_eq!(entry["held"], false);
    assert_eq!(entry["after_unclean_stop"], false);
    assert_eq!(entry["unclean_stop"], Value::Null);
    assert!(entry["reason"].as_str().is_some(), "{entry}");
    assert_eq!(result_names(&results), ["checking", "new_session"]);
    assert_eq!(results[0]["continued"], false);
    let actions = actions_of(&mut client, &agent).await;
    let user_start = actions
        .iter()
        .rev()
        .find(|entry| entry["action"] == "start" && entry["actor"] == "user")
        .expect("a start by the user");
    assert!(user_start["detail"].as_str().is_some(), "{actions:?}");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_user_stop_inside_the_check_period_ends_the_watch_without_a_new_session() {
    let office = open_office().await;
    office.clis.add(
        "claude",
        &claude_body(&office, KEEPS_RUNNING, KEEPS_RUNNING),
    );
    let mut client = office.client().await;
    let agent = interrupted_agent(&mut client, &office, "Ana", "claude").await;

    let request = send_resume(&mut client, &[&agent]).await;
    let checking = next_result_of(&mut client, &agent).await;
    let stop = client.send("agent.stop", json!({"agent_id": agent})).await;
    let results = results_of(&mut client, &agent).await;

    answered_ok(&mut client, request).await;
    answered_ok(&mut client, stop).await;
    assert_eq!(checking["result"], "checking");
    assert_eq!(result_names(&results), ["continued"]);
    let actions = actions_of(&mut client, &agent).await;
    assert_eq!(office_starts(&actions), Vec::<&Value>::new(), "{actions:?}");
    office.serving.stop().await;
}

#[tokio::test]
async fn an_agent_that_is_no_longer_interrupted_is_skipped() {
    let office = open_office().await;
    office.clis.add("plain-cli", "echo ready");
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "plain-cli", None).await;
    client.start(&agent).await;
    client.wait_for_status(&agent, "exited").await;

    let request = send_resume(&mut client, &[&agent]).await;
    let results = results_of(&mut client, &agent).await;
    answered_ok(&mut client, request).await;

    assert_eq!(result_names(&results), ["skipped"]);
    assert!(
        results[0]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("no longer interrupted"))
    );
    office.serving.stop().await;
}

#[tokio::test]
async fn members_start_before_their_lead_and_the_lead_before_the_ceo() {
    let office = open_office().await;
    office.clis.add("plain-cli", "echo ready");
    let mut client = office.client().await;
    let ceo = client
        .ask(
            "agent.create",
            json!({"name": "Chief", "command": "plain-cli", "working_folder": office.work_path(), "role": {"kind": "ceo"}}),
        )
        .await;
    let team = client
        .ask(
            "team.create",
            json!({"name": "Build", "lead": {"name": "Lead", "command": "plain-cli", "working_folder": office.work_path()}}),
        )
        .await;
    let ceo = ceo["result"]["id"].as_str().expect("ceo").to_owned();
    let lead = team["result"]["lead_agent_id"]
        .as_str()
        .expect("lead")
        .to_owned();
    let member = client
        .ask(
            "agent.create",
            json!({"name": "Member", "command": "plain-cli", "working_folder": office.work_path(), "role": {"kind": "member", "parent_agent_id": lead}}),
        )
        .await;
    let member = member["result"]["id"].as_str().expect("member").to_owned();
    for agent in [&ceo, &lead, &member] {
        mark_interrupted(&office, agent);
    }

    let request = send_resume(&mut client, &[&ceo, &lead, &member]).await;
    let answer = answered_ok(&mut client, request).await;

    assert_eq!(answer["order"], json!([member, lead, ceo]));
    office.serving.stop().await;
}

#[tokio::test]
async fn an_empty_or_unknown_field_request_is_refused() {
    let office = open_office().await;
    let mut client = office.client().await;

    let empty = client.ask("resume.start", json!({"agent_ids": []})).await;
    let unknown = client.ask("resume.plan", json!({"all": true})).await;

    assert_eq!(error_text(&empty).0, "invalid");
    assert_eq!(error_text(&unknown).0, "invalid");
    office.serving.stop().await;
}
