// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! The Needs you queue at protocol level (0008, T4): `needs_you.list`,
//! `needs_you.dismiss`, and `needs_you.changed`, against a real server and a
//! real client. Attention events that need no process are published to the
//! hub the way the status bridge publishes them, as `notifications.rs` does.

mod support;

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use agent_domain::enums::AgentStatus;
use agent_presentation::agent_status_event;
use office_harnessd::bootstrap;
use office_harnessd::server::{Server, ServerOptions};
use serde_json::{Value, json};
use support::entries_under;
use support::office_client::{Client, Office, error_text};
use support::wire_client::WireClient;

const QUIET_PERIOD_MS: u64 = 400;
const POLL: Duration = Duration::from_millis(25);
const WAIT_LIMIT: Duration = Duration::from_secs(20);
/// SC-003: the change reaches a connected client within this time.
const EVENT_DEADLINE: Duration = Duration::from_secs(2);
/// Keeps two events from sharing a millisecond, so the order by first event
/// time does not fall to the name tie-break.
const EVENT_SPACING: Duration = Duration::from_millis(10);
/// How long the busy agent of SC-005 is watched, and the pause between its
/// writes (shorter than the quiet period).
const BUSY_WATCH: Duration = Duration::from_millis(1500);
const BUSY_SCRIPT: &str =
    "i=0; while [ $i -lt 100 ]; do echo tick; sleep 0.1; i=$((i+1)); done; sleep 30";
const AGENT_COUNT: usize = 30;
const NO_ITEM_MESSAGE: &str = "no needs-you item for the agent";

fn publish_status(office: &Office, agent: &str, status: AgentStatus) {
    office
        .serving
        .office
        .services
        .events
        .publish(&agent_status_event(
            agent.parse().expect("agent id"),
            status,
            None,
        ));
}

/// Publishes the events one after the other, spaced in time.
async fn publish_spaced(office: &Office, events: &[(&str, AgentStatus)]) {
    for (agent, status) in events {
        publish_status(office, agent, *status);
        tokio::time::sleep(EVENT_SPACING).await;
    }
}

fn now_unix_ms() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_millis(),
    )
    .expect("fits")
}

async fn list_items(client: &mut Client) -> Vec<Value> {
    let answer = client.ask("needs_you.list", json!({})).await;
    assert_eq!(answer["ok"], true, "{answer}");
    answer["result"]["items"].as_array().expect("items").clone()
}

/// Waits until the list holds `count` items, then returns them.
async fn wait_for_items(client: &mut Client, count: usize) -> Vec<Value> {
    let deadline = Instant::now() + WAIT_LIMIT;
    loop {
        let items = list_items(client).await;
        if items.len() == count {
            return items;
        }
        assert!(
            Instant::now() < deadline,
            "the list never held {count} items: {items:?}"
        );
        tokio::time::sleep(POLL).await;
    }
}

fn agents_of(items: &[Value]) -> Vec<&str> {
    items
        .iter()
        .map(|item| item["agent_id"].as_str().expect("agent id"))
        .collect()
}

fn kinds_of(items: &[Value]) -> Vec<&str> {
    items
        .iter()
        .map(|item| item["kind"].as_str().expect("kind"))
        .collect()
}

/// Reads events from a client that only listens, until one holds `count`
/// items. A client that also asks drops the events that come before an
/// answer.
async fn next_changed_with(watcher: &mut Client, count: usize) -> Value {
    next_changed_where(watcher, |items| {
        items.as_array().expect("items").len() == count
    })
    .await
}

/// The first `needs_you.changed` that carries exactly `items`; the earlier
/// events of the same test carry the lists on the way there.
async fn next_changed_to(watcher: &mut Client, items: &Value) -> Value {
    next_changed_where(watcher, |event_items| event_items == items).await
}

async fn next_changed_where(watcher: &mut Client, matches: impl Fn(&Value) -> bool) -> Value {
    tokio::time::timeout(EVENT_DEADLINE, async {
        loop {
            let event = watcher.next_event("needs_you.changed").await;
            if matches(&event["items"]) {
                return event;
            }
        }
    })
    .await
    .expect("needs_you.changed within the deadline")
}

/// An office with one connected client and the named agents.
async fn office_with_agents(names: &[&str]) -> (Office, Client, Vec<String>) {
    let office = Office::open().await;
    let mut client = office.client().await;
    let mut agents = Vec::new();
    for name in names {
        agents.push(client.create_agent(&office, name, "false", None).await);
    }
    (office, client, agents)
}

#[tokio::test]
async fn thirty_agents_with_five_events_list_exactly_those_five_in_order() {
    let names: Vec<String> = (0..AGENT_COUNT)
        .map(|index| format!("Agent{index:02}"))
        .collect();
    let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let (office, mut client, agents) = office_with_agents(&name_refs).await;

    // Published oldest first: the failed pair, then the waiting pair, then
    // the exited one, whatever the publish order.
    publish_spaced(
        &office,
        &[
            (&agents[3], AgentStatus::Exited),
            (&agents[7], AgentStatus::WaitingForInput),
            (&agents[11], AgentStatus::Failed),
            (&agents[19], AgentStatus::WaitingForInput),
            (&agents[23], AgentStatus::Failed),
        ],
    )
    .await;

    let items = wait_for_items(&mut client, 5).await;
    assert_eq!(
        agents_of(&items),
        [
            agents[11].as_str(),
            agents[23].as_str(),
            agents[7].as_str(),
            agents[19].as_str(),
            agents[3].as_str(),
        ]
    );
    assert_eq!(
        kinds_of(&items),
        [
            "failed",
            "failed",
            "waiting_for_input",
            "waiting_for_input",
            "exited"
        ]
    );
    let waiting = &items[2];
    assert_eq!(waiting["detail"], QUIET_PERIOD_MS.to_string());
    assert!(waiting["first_event_at"].is_u64());
    assert!(waiting["latest_at"].is_u64());
    assert!(waiting["origin_notification_id"].is_string());
    office.serving.stop().await;
}

#[tokio::test]
async fn a_change_reaches_a_connected_client_within_two_seconds() {
    let (office, mut client, agents) = office_with_agents(&["Ana"]).await;

    publish_status(&office, &agents[0], AgentStatus::Exited);

    let event = tokio::time::timeout(EVENT_DEADLINE, client.next_event("needs_you.changed"))
        .await
        .expect("needs_you.changed within the deadline");
    assert_eq!(event["items"][0]["agent_id"], agents[0].as_str());
    assert_eq!(event["items"][0]["kind"], "exited");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_wait_is_announced_after_the_quiet_period_and_within_two_seconds_of_it() {
    let office = Office::open().await;
    office.clis.add("quiet-cli", "echo ready; sleep 30");
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "quiet-cli", None).await;

    let started_at = Instant::now();
    client.start(&agent).await;
    let event = tokio::time::timeout(
        Duration::from_millis(QUIET_PERIOD_MS) + EVENT_DEADLINE,
        client.next_event("needs_you.changed"),
    )
    .await
    .expect("needs_you.changed within the deadline");

    assert!(
        started_at.elapsed() >= Duration::from_millis(QUIET_PERIOD_MS),
        "the item came before the quiet period ended"
    );
    assert_eq!(event["items"][0]["kind"], "waiting_for_input");
    assert_eq!(event["items"][0]["detail"], QUIET_PERIOD_MS.to_string());
    office.serving.stop().await;
}

#[tokio::test]
async fn events_while_no_client_is_connected_list_with_their_first_event_times() {
    let (office, first, agents) = office_with_agents(&["Ana", "Raj", "Kim"]).await;
    drop(first);

    let before = now_unix_ms();
    publish_spaced(
        &office,
        &[
            (&agents[0], AgentStatus::Exited),
            (&agents[1], AgentStatus::Failed),
            (&agents[2], AgentStatus::WaitingForInput),
        ],
    )
    .await;
    let mut second = office.client().await;

    let items = wait_for_items(&mut second, 3).await;
    let after = now_unix_ms();
    assert_eq!(
        agents_of(&items),
        [agents[1].as_str(), agents[2].as_str(), agents[0].as_str()]
    );
    for item in &items {
        let first_event_at = item["first_event_at"].as_u64().expect("first event time");
        assert!((before..=after).contains(&first_event_at), "{item}");
    }
    office.serving.stop().await;
}

#[tokio::test]
async fn an_agent_that_keeps_writing_and_rings_no_bell_gets_no_item() {
    let office = Office::open().await;
    office.clis.add("busy-cli", BUSY_SCRIPT);
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "busy-cli", None).await;
    client.start(&agent).await;

    let watch_ends = Instant::now() + BUSY_WATCH;
    while Instant::now() < watch_ends {
        assert_eq!(list_items(&mut client).await, Vec::<Value>::new());
        tokio::time::sleep(POLL).await;
    }

    assert_eq!(client.status_of(&agent).await, "working");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_bell_makes_a_bell_item() {
    let office = Office::open().await;
    office
        .clis
        .add("bell-cli", "echo hi; sleep 0.1; printf '\\a'; sleep 30");
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "bell-cli", None).await;
    client.start(&agent).await;

    // The first change is the bell: the quiet period ends after it, so the
    // wait can only come as a later change.
    let event = tokio::time::timeout(WAIT_LIMIT, client.next_event("needs_you.changed"))
        .await
        .expect("needs_you.changed within the limit");

    assert_eq!(event["items"][0]["agent_id"], agent.as_str());
    assert_eq!(event["items"][0]["kind"], "bell");
    assert_eq!(event["items"][0]["detail"], Value::Null);
    office.serving.stop().await;
}

#[tokio::test]
async fn a_failed_command_shows_the_text_the_status_frame_carries() {
    let office = Office::open().await;
    office.clis.add("failing-cli", "exit 2");
    let mut client = office.client().await;
    let agent = client
        .create_agent(&office, "Ana", "failing-cli", None)
        .await;
    client.start(&agent).await;

    let items = wait_for_items(&mut client, 1).await;

    assert_eq!(kinds_of(&items), ["failed"]);
    assert_eq!(items[0]["detail"], "the command ended with exit code 2");
    office.serving.stop().await;
}

#[tokio::test]
async fn reading_the_origin_notification_clears_the_item_and_tells_the_clients() {
    let (office, mut client, agents) = office_with_agents(&["Ana"]).await;
    let mut watcher = office.client().await;
    publish_status(&office, &agents[0], AgentStatus::Exited);
    let items = wait_for_items(&mut client, 1).await;
    let origin = items[0]["origin_notification_id"].clone();
    assert!(origin.is_string(), "{}", items[0]);

    let read = client
        .ask("notification.read", json!({"notification_id": origin}))
        .await;
    assert_eq!(read["ok"], true, "{read}");

    next_changed_with(&mut watcher, 0).await;
    assert_eq!(list_items(&mut client).await, Vec::<Value>::new());
    office.serving.stop().await;
}

#[tokio::test]
async fn read_agent_clears_the_item_and_tells_the_clients() {
    let (office, mut client, agents) = office_with_agents(&["Ana"]).await;
    let mut watcher = office.client().await;
    publish_status(&office, &agents[0], AgentStatus::Failed);
    wait_for_items(&mut client, 1).await;

    let read = client
        .ask("notification.read_agent", json!({"agent_id": agents[0]}))
        .await;
    assert_eq!(read["ok"], true, "{read}");

    next_changed_with(&mut watcher, 0).await;
    assert_eq!(list_items(&mut client).await, Vec::<Value>::new());
    office.serving.stop().await;
}

#[tokio::test]
async fn a_dismiss_answers_the_rest_and_marks_nothing_read() {
    let (office, mut client, agents) = office_with_agents(&["Ana", "Raj"]).await;
    let mut watcher = office.client().await;
    publish_spaced(
        &office,
        &[
            (&agents[0], AgentStatus::Failed),
            (&agents[1], AgentStatus::Exited),
        ],
    )
    .await;
    wait_for_items(&mut client, 2).await;

    let dismissed = client
        .ask("needs_you.dismiss", json!({"agent_id": agents[0]}))
        .await;

    assert_eq!(dismissed["ok"], true, "{dismissed}");
    let rest = dismissed["result"]["items"].as_array().expect("items");
    assert_eq!(agents_of(rest), [agents[1].as_str()]);
    next_changed_to(&mut watcher, &dismissed["result"]["items"]).await;
    let unread = client
        .ask("notification.list", json!({"unread_only": true}))
        .await;
    assert_eq!(unread["result"].as_array().expect("rows").len(), 2);
    office.serving.stop().await;
}

#[tokio::test]
async fn dismiss_refuses_an_agent_without_an_item_a_bad_id_and_an_unknown_field() {
    let (office, mut client, agents) = office_with_agents(&["Ana"]).await;

    let none = client
        .ask("needs_you.dismiss", json!({"agent_id": agents[0]}))
        .await;
    assert_eq!(error_text(&none), ("not_found", NO_ITEM_MESSAGE));

    let bad = client
        .ask("needs_you.dismiss", json!({"agent_id": "nope"}))
        .await;
    assert_eq!(
        error_text(&bad),
        ("invalid", "agent_id is not a valid identifier: nope")
    );

    let misspelt = client
        .ask("needs_you.dismiss", json!({"agent_idd": agents[0]}))
        .await;
    assert_eq!(
        error_text(&misspelt),
        ("invalid", "agent_idd is not a field of needs_you.dismiss")
    );

    let extra = client
        .ask(
            "needs_you.dismiss",
            json!({"agent_id": agents[0], "all": true}),
        )
        .await;
    assert_eq!(
        error_text(&extra),
        ("invalid", "all is not a field of needs_you.dismiss")
    );

    let list_extra = client.ask("needs_you.list", json!({"limit": 1})).await;
    assert_eq!(
        error_text(&list_extra),
        ("invalid", "limit is not a field of needs_you.list")
    );
    office.serving.stop().await;
}

#[tokio::test]
async fn working_clears_the_item_and_interrupted_keeps_it() {
    let (office, mut client, agents) = office_with_agents(&["Ana"]).await;
    publish_status(&office, &agents[0], AgentStatus::Failed);
    let first = wait_for_items(&mut client, 1).await;

    // Frames reach the bridge in order, so the exited frame that follows
    // tells that the interrupted frame was handled. An item that the
    // interrupted frame had cleared would start again with a later first
    // event time.
    publish_status(&office, &agents[0], AgentStatus::Interrupted);
    tokio::time::sleep(EVENT_SPACING).await;
    publish_status(&office, &agents[0], AgentStatus::Exited);
    let kept = loop {
        let items = wait_for_items(&mut client, 1).await;
        if items[0]["kind"] == "exited" {
            break items;
        }
        tokio::time::sleep(POLL).await;
    };
    assert_eq!(kept[0]["first_event_at"], first[0]["first_event_at"]);

    publish_status(&office, &agents[0], AgentStatus::Working);
    wait_for_items(&mut client, 0).await;
    office.serving.stop().await;
}

#[tokio::test]
async fn deleting_the_agent_clears_its_item() {
    let (office, mut client, agents) = office_with_agents(&["Ana", "Raj"]).await;
    publish_spaced(
        &office,
        &[
            (&agents[0], AgentStatus::Failed),
            (&agents[1], AgentStatus::Failed),
        ],
    )
    .await;
    wait_for_items(&mut client, 2).await;

    let deleted = client
        .ask("agent.delete", json!({"agent_id": agents[0]}))
        .await;
    assert_eq!(deleted["ok"], true, "{deleted}");

    let items = wait_for_items(&mut client, 1).await;
    assert_eq!(agents_of(&items), [agents[1].as_str()]);
    office.serving.stop().await;
}

#[tokio::test]
async fn items_survive_a_server_restart_and_the_interrupted_mark() {
    let office = Office::open().await;
    office.clis.add("quiet-cli", "echo ready; sleep 30");
    let mut client = office.client().await;
    let failed = client.create_agent(&office, "Ana", "false", None).await;
    let waiting = client.create_agent(&office, "Raj", "quiet-cli", None).await;
    client.start(&waiting).await;
    publish_status(&office, &failed, AgentStatus::Failed);
    let before = wait_for_items(&mut client, 2).await;
    drop(client);

    // The restart marks the running agent interrupted, which must keep its
    // item.
    let stopped = office.serving.stop().await;
    let services = bootstrap::start_with(&stopped.config, &stopped.log, office.clis.login_setup())
        .expect("restart");
    let server = Server::bind(
        &stopped.config,
        &services,
        &stopped.log,
        ServerOptions::default(),
    )
    .await
    .expect("bind");
    let port = server.port();
    tokio::spawn(async move {
        server
            .serve(std::future::pending::<()>())
            .await
            .expect("serve");
    });
    let mut again = Client::new(WireClient::authenticate_ok(port, &stopped.token()).await);

    let after = list_items(&mut again).await;
    assert_eq!(after, before);
    assert_eq!(again.status_of(&waiting).await, "interrupted");
}

/// PB-2: the items come from the status and bell events that exist already;
/// listing, dismissing, and reading them writes nothing in the home folders
/// the CLIs read or in the project folder.
#[tokio::test]
async fn needs_you_writes_nothing_in_a_cli_configuration_or_the_project_folder() {
    let office = Office::open().await;
    office
        .clis
        .add("bell-cli", "echo hi; sleep 0.1; printf '\\a'; sleep 30");
    office.clis.add("failing-cli", "exit 2");
    let fake_folders = office.clis.tool_folder().parent().expect("fake folders");
    let watched = [fake_folders, office.work_folder.path()];
    let mut client = office.client().await;
    let bell = client.create_agent(&office, "Ana", "bell-cli", None).await;
    let failing = client
        .create_agent(&office, "Raj", "failing-cli", None)
        .await;
    let before = entries_under(&watched);

    client.start(&bell).await;
    client.start(&failing).await;
    let items = wait_for_items(&mut client, 2).await;
    assert_eq!(kinds_of(&items).len(), 2);
    let dismissed = client
        .ask("needs_you.dismiss", json!({"agent_id": bell}))
        .await;
    assert_eq!(dismissed["ok"], true, "{dismissed}");
    let read = client
        .ask("notification.read_agent", json!({"agent_id": failing}))
        .await;
    assert_eq!(read["ok"], true, "{read}");
    wait_for_items(&mut client, 0).await;

    assert_eq!(entries_under(&watched), before);
    office.serving.stop().await;
}
