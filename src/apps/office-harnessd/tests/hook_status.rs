// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! 0024 T2: the status of a session follows the reports of its CLI. The test
//! CLIs are plain shell scripts; the test posts the reports that the hooks of
//! `claude` would, so that each rule is driven the way the CLI drives it.

mod support;

use std::time::{Duration, Instant};

use serde_json::json;
use support::fake_cli::FakeClis;
use support::hook_client::HookPoster;
use support::office_client::{Client, Office};

const QUIET_PERIOD_MS: u64 = 400;
const POLL: Duration = Duration::from_millis(25);
/// The exact status shows within a second of the report (0024/FR-012).
const REPORT_LIMIT: Duration = Duration::from_secs(1);
/// "hi" and Enter, as the CLI's terminal receives a person's line.
const TYPED_LINE: &str = "aGkK";
/// Quiet after its first line: waiting for input.
const WAITING_CLI: &str = "echo ready; sleep 60";
/// Prints a line on its own every 0.7 s, three times, after the test has
/// made the session exact.
const REDRAWING_CLI: &str = "echo ready; sleep 2; i=0; while [ $i -lt 3 ]; do echo redraw; sleep 0.7; i=$((i+1)); done; sleep 60";
/// Waits for a line, then prints once and goes quiet: a turn whose report
/// was lost.
const ANSWERING_CLI: &str = "echo ready; read line; echo busy; sleep 60";

async fn open_agent(script: &str) -> (Office, Client, String, HookPoster) {
    let office = Office::open_with_quiet_period(FakeClis::new(), QUIET_PERIOD_MS).await;
    office.clis.add("test-cli", script);
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "test-cli", None).await;
    client.start(&agent).await;
    client.wait_for_status(&agent, "waiting_for_input").await;
    let poster = HookPoster::for_agent(&office, &agent);
    (office, client, agent, poster)
}

async fn wait_for_status_within(client: &mut Client, agent: &str, wanted: &str, limit: Duration) {
    let deadline = Instant::now() + limit;
    while client.status_of(agent).await != wanted {
        assert!(
            Instant::now() < deadline,
            "the agent did not become {wanted} within {limit:?}"
        );
        tokio::time::sleep(POLL).await;
    }
}

async fn notifications_of(client: &mut Client, event: &str) -> usize {
    let answer = client.ask("notification.list", json!({})).await;
    answer["result"]
        .as_array()
        .expect("notifications")
        .iter()
        .filter(|row| row["event"] == event)
        .count()
}

async fn type_line(client: &mut Client, agent: &str) {
    let typed = client
        .ask(
            "terminal.input",
            json!({"agent_id": agent, "data": TYPED_LINE}),
        )
        .await;
    assert_eq!(typed["ok"], true, "{typed}");
}

#[tokio::test]
async fn a_turn_works_from_its_start_to_its_stop_within_a_second_of_each_report() {
    let (office, mut client, agent, poster) = open_agent(WAITING_CLI).await;
    assert_eq!(poster.report("UserPromptSubmit", "").await, 200);
    wait_for_status_within(&mut client, &agent, "working", REPORT_LIMIT).await;
    assert_eq!(poster.report("Stop", r#""prompt":"words""#).await, 200);
    wait_for_status_within(&mut client, &agent, "waiting_for_input", REPORT_LIMIT).await;
    office.serving.stop().await;
}

#[tokio::test]
async fn output_of_a_waiting_exact_session_makes_no_change_and_no_notification() {
    let (office, mut client, agent, poster) = open_agent(REDRAWING_CLI).await;
    poster.report("UserPromptSubmit", "").await;
    poster.report("Stop", "").await;
    wait_for_status_within(&mut client, &agent, "waiting_for_input", REPORT_LIMIT).await;
    let before = notifications_of(&mut client, "waiting_for_input").await;
    let watch_ends = Instant::now() + Duration::from_millis(3500);
    while Instant::now() < watch_ends {
        assert_eq!(client.status_of(&agent).await, "waiting_for_input");
        tokio::time::sleep(POLL).await;
    }
    assert_eq!(
        notifications_of(&mut client, "waiting_for_input").await,
        before
    );
    office.serving.stop().await;
}

#[tokio::test]
async fn a_permission_ask_waits_inside_the_turn_and_a_person_input_ends_it() {
    let (office, mut client, agent, poster) = open_agent(WAITING_CLI).await;
    poster.report("UserPromptSubmit", "").await;
    wait_for_status_within(&mut client, &agent, "working", REPORT_LIMIT).await;
    poster.report("PermissionRequest", "").await;
    wait_for_status_within(&mut client, &agent, "waiting_for_input", REPORT_LIMIT).await;
    type_line(&mut client, &agent).await;
    wait_for_status_within(&mut client, &agent, "working", REPORT_LIMIT).await;
    office.serving.stop().await;
}

#[tokio::test]
async fn a_lost_stop_turns_the_session_late_and_the_quiet_period_decides_again() {
    let (office, mut client, agent, poster) = open_agent(ANSWERING_CLI).await;
    poster.report("UserPromptSubmit", "").await;
    wait_for_status_within(&mut client, &agent, "working", REPORT_LIMIT).await;
    type_line(&mut client, &agent).await;
    // "busy" is one burst; one quiet period after it the open turn is silent.
    wait_for_status_within(
        &mut client,
        &agent,
        "waiting_for_input",
        Duration::from_secs(10),
    )
    .await;
    office.serving.stop().await;
}

#[tokio::test]
async fn reports_of_a_helper_sub_agent_change_nothing() {
    let (office, mut client, agent, poster) = open_agent(WAITING_CLI).await;
    assert_eq!(
        poster
            .report("UserPromptSubmit", r#""agent_id":"helper""#)
            .await,
        200
    );
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(client.status_of(&agent).await, "waiting_for_input");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_report_for_an_agent_without_a_running_session_changes_nothing() {
    let (office, mut client, agent, poster) = open_agent(WAITING_CLI).await;
    let stopped = client.ask("agent.stop", json!({"agent_id": agent})).await;
    assert_eq!(stopped["ok"], true, "{stopped}");
    client.wait_for_status(&agent, "stopped").await;
    assert_eq!(poster.report("UserPromptSubmit", "").await, 200);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(client.status_of(&agent).await, "stopped");
    office.serving.stop().await;
}
