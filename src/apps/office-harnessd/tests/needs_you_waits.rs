// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! One wait gives one notification and one Needs you item (0022, US2): a CLI
//! that redraws its screen on its own is no new wait, and a stretch of work
//! past the work bound is. The quiet period and the work bound are short, so
//! a test takes seconds.

mod support;

use std::time::{Duration, Instant};

use office_harnessd::server::ServerOptions;
use serde_json::{Value, json};
use support::fake_cli::FakeClis;
use support::office_client::{Client, Office};

const QUIET_PERIOD_MS: u64 = 400;
/// With the quiet period, the work bound is one second.
const WORK_BOUND_EXTRA: Duration = Duration::from_millis(600);
const POLL: Duration = Duration::from_millis(25);
const WAIT_LIMIT: Duration = Duration::from_secs(20);
/// "hi" and Enter, as the CLI's terminal receives a person's line.
const TYPED_LINE: &str = "aGkK";
/// The CLI answers once, then prints a line on its own every 0.7 s: each
/// line is a redraw that shows `working` for one quiet period.
const REDRAWING_CLI: &str =
    "echo ready; i=0; while [ $i -lt 3 ]; do sleep 0.7; echo redraw; i=$((i+1)); done; sleep 30";
/// How long the redrawing CLI is watched: after its last redraw has gone
/// quiet again.
const REDRAW_WATCH: Duration = Duration::from_millis(3000);
/// Waits for a line, then answers it.
const ANSWERING_CLI: &str = "echo ready; read line; echo got; sleep 30";
/// Waits for a line, then works for 2.5 s, over the work bound.
const LONG_WORK_CLI: &str = "echo ready; read line; i=0; while [ $i -lt 25 ]; do echo busy; \
                             sleep 0.1; i=$((i+1)); done; sleep 30";
/// Waits, then works on its own for 3 s without any input.
const SELF_WORKING_CLI: &str = "echo ready; sleep 1; i=0; while [ $i -lt 30 ]; do echo busy; \
                                sleep 0.1; i=$((i+1)); done; sleep 30";
const BELL_AFTER_WAIT_CLI: &str = "echo ready; sleep 1; printf '\\a'; sleep 30";
/// Waits, redraws once, then answers a line: a line typed during the
/// redraw's `working` reaches an agent that shows `working`.
const REDRAW_THEN_ANSWER_CLI: &str =
    "echo ready; sleep 1; echo redraw; read line; echo got; sleep 30";
/// Well under the one-second work bound, so only the input can have
/// removed the item in this time.
const INPUT_CLEAR_LIMIT: Duration = Duration::from_millis(300);

async fn open_office(script: &str) -> (Office, Client, String) {
    let office = Office::open_with_options_and_quiet_period(
        FakeClis::new(),
        ServerOptions {
            work_bound_extra: WORK_BOUND_EXTRA,
            ..ServerOptions::default()
        },
        QUIET_PERIOD_MS,
    )
    .await;
    office.clis.add("test-cli", script);
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "test-cli", None).await;
    client.start(&agent).await;
    (office, client, agent)
}

async fn items(client: &mut Client) -> Vec<Value> {
    let answer = client.ask("needs_you.list", json!({})).await;
    assert_eq!(answer["ok"], true, "{answer}");
    answer["result"]["items"].as_array().expect("items").clone()
}

async fn wait_for_items(client: &mut Client, count: usize) -> Vec<Value> {
    let deadline = Instant::now() + WAIT_LIMIT;
    loop {
        let listed = items(client).await;
        if listed.len() == count {
            return listed;
        }
        assert!(
            Instant::now() < deadline,
            "the list never held {count} items: {listed:?}"
        );
        tokio::time::sleep(POLL).await;
    }
}

fn first_event_at(item: &Value) -> u64 {
    item["first_event_at"].as_u64().expect("first event time")
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
async fn a_redrawing_cli_gives_one_notification_and_one_item_with_one_wait_time() {
    let (office, mut client, _agent) = open_office(REDRAWING_CLI).await;
    let first = wait_for_items(&mut client, 1).await;

    let watch_ends = Instant::now() + REDRAW_WATCH;
    while Instant::now() < watch_ends {
        let listed = items(&mut client).await;
        assert_eq!(listed.len(), 1, "{listed:?}");
        assert_eq!(first_event_at(&listed[0]), first_event_at(&first[0]));
        tokio::time::sleep(POLL).await;
    }

    assert_eq!(notifications_of(&mut client, "waiting_for_input").await, 1);
    office.serving.stop().await;
}

#[tokio::test]
async fn a_dismissed_item_stays_gone_while_the_cli_redraws() {
    let (office, mut client, agent) = open_office(REDRAWING_CLI).await;
    client.dismiss_needs_you(&agent).await;

    let watch_ends = Instant::now() + REDRAW_WATCH;
    while Instant::now() < watch_ends {
        assert_eq!(items(&mut client).await, Vec::<Value>::new());
        tokio::time::sleep(POLL).await;
    }

    assert_eq!(notifications_of(&mut client, "waiting_for_input").await, 1);
    office.serving.stop().await;
}

#[tokio::test]
async fn input_into_an_agent_with_an_item_removes_it_and_the_return_gives_a_new_one() {
    let (office, mut client, agent) = open_office(ANSWERING_CLI).await;
    let before = wait_for_items(&mut client, 1).await;

    type_line(&mut client, &agent).await;
    wait_for_items(&mut client, 0).await;
    let after = wait_for_items(&mut client, 1).await;

    assert!(first_event_at(&after[0]) > first_event_at(&before[0]));
    assert_eq!(notifications_of(&mut client, "waiting_for_input").await, 2);
    office.serving.stop().await;
}

#[tokio::test]
async fn work_past_the_bound_after_a_dismiss_gives_one_item_and_one_notification_at_the_return() {
    let (office, mut client, agent) = open_office(LONG_WORK_CLI).await;
    client.dismiss_needs_you(&agent).await;

    type_line(&mut client, &agent).await;
    let returned = wait_for_items(&mut client, 1).await;

    assert_eq!(returned[0]["kind"], "waiting_for_input");
    assert_eq!(notifications_of(&mut client, "waiting_for_input").await, 2);
    office.serving.stop().await;
}

#[tokio::test]
async fn the_item_leaves_at_the_bound_while_the_agent_still_works() {
    let (office, mut client, agent) = open_office(SELF_WORKING_CLI).await;
    let before = wait_for_items(&mut client, 1).await;

    wait_for_items(&mut client, 0).await;
    assert_eq!(client.status_of(&agent).await, "working");

    let after = wait_for_items(&mut client, 1).await;
    assert!(first_event_at(&after[0]) > first_event_at(&before[0]));
    assert_eq!(notifications_of(&mut client, "waiting_for_input").await, 2);
    office.serving.stop().await;
}

#[tokio::test]
async fn a_bell_after_the_wait_still_makes_a_bell_item_that_a_later_redraw_leaves_alone() {
    let (office, mut client, _agent) = open_office(BELL_AFTER_WAIT_CLI).await;
    let waiting = wait_for_items(&mut client, 1).await;

    let deadline = Instant::now() + WAIT_LIMIT;
    while items(&mut client).await[0]["kind"] != "bell" {
        assert!(Instant::now() < deadline, "the bell never reached the item");
        tokio::time::sleep(POLL).await;
    }
    // The bell's output is a redraw: it goes quiet again after the quiet
    // period and must not turn the item back into a wait.
    tokio::time::sleep(Duration::from_millis(QUIET_PERIOD_MS * 2)).await;

    let listed = items(&mut client).await;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["kind"], "bell");
    assert_eq!(first_event_at(&listed[0]), first_event_at(&waiting[0]));
    assert_eq!(notifications_of(&mut client, "bell").await, 1);
    assert_eq!(notifications_of(&mut client, "waiting_for_input").await, 1);
    office.serving.stop().await;
}

#[tokio::test]
async fn input_while_a_redraw_shows_working_removes_the_item_and_the_return_gives_a_new_one() {
    let (office, mut client, agent) = open_office(REDRAW_THEN_ANSWER_CLI).await;
    let before = wait_for_items(&mut client, 1).await;
    client.wait_for_status(&agent, "working").await;
    assert_eq!(items(&mut client).await.len(), 1, "a redraw keeps the item");

    type_line(&mut client, &agent).await;
    let typed_at = Instant::now();
    while !items(&mut client).await.is_empty() {
        assert!(
            typed_at.elapsed() < INPUT_CLEAR_LIMIT,
            "the input did not remove the item"
        );
        tokio::time::sleep(POLL).await;
    }

    let after = wait_for_items(&mut client, 1).await;
    assert!(first_event_at(&after[0]) > first_event_at(&before[0]));
    assert_eq!(notifications_of(&mut client, "waiting_for_input").await, 2);
    office.serving.stop().await;
}
