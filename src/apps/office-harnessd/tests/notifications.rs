// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! T3 at protocol level: the `notification.*` requests, the
//! `notification.created` event, and the unread listing after the events
//! happened with no client connected (FR-007, SC-005 shape with 3 agents).

mod support;

use std::time::Duration;

use agent_domain::enums::AgentStatus;
use agent_presentation::agent_status_event;
use kernel::AgentId;
use serde_json::{Value, json};
use support::office_client::{Office, error_text};

const POLL: Duration = Duration::from_millis(25);

/// The events happen while every client is gone, like the app being closed:
/// the frames go to the hub the same way the status bridge publishes them.
fn events_with_no_client(office: &Office, events: &[(AgentId, AgentStatus)]) {
    for (agent_id, status) in events {
        office
            .serving
            .office
            .services
            .events
            .publish(&agent_status_event(*agent_id, *status, None));
    }
}

fn names_of(listed: &Value) -> Vec<String> {
    listed
        .as_array()
        .expect("notifications")
        .iter()
        .map(|row| row["agent_name"].as_str().expect("name").to_owned())
        .collect()
}

fn unread_of(listed: &Value) -> Vec<String> {
    listed
        .as_array()
        .expect("notifications")
        .iter()
        .filter(|row| row["read_at"].is_null())
        .map(|row| row["agent_name"].as_str().expect("name").to_owned())
        .collect()
}

#[tokio::test]
async fn events_without_a_client_list_as_unread_after_a_client_connects() {
    let office = Office::open().await;
    let mut first = office.client().await;
    let ana = first.create_agent(&office, "Ana", "false", None).await;
    let raj = first.create_agent(&office, "Raj", "false", None).await;
    let kim = first.create_agent(&office, "Kim", "false", None).await;
    drop(first);

    let parse = |id: &str| id.parse::<AgentId>().expect("agent id");
    events_with_no_client(
        &office,
        &[
            (parse(&ana), AgentStatus::WaitingForInput),
            (parse(&raj), AgentStatus::Exited),
            (parse(&kim), AgentStatus::Failed),
            // The user's own stop records nothing (US2 scenario 7).
            (parse(&ana), AgentStatus::Stopped),
        ],
    );

    // A client connects only after the events, like an app that was closed
    // while the agents changed.
    let mut second = office.client().await;
    let listed = loop {
        let answer = second.ask("notification.list", json!({})).await;
        assert_eq!(answer["ok"], true, "{answer}");
        if answer["result"]
            .as_array()
            .is_some_and(|rows| rows.len() == 3)
        {
            break answer;
        }
        tokio::time::sleep(POLL).await;
    };
    let rows = listed["result"].as_array().expect("rows");
    // Newest first, and the stopped frame is absent.
    assert_eq!(names_of(&listed["result"]), ["Kim", "Raj", "Ana"]);
    let kinds: Vec<&str> = rows
        .iter()
        .map(|row| row["event"].as_str().expect("event"))
        .collect();
    assert_eq!(kinds, ["failed", "exited", "waiting_for_input"]);
    assert!(
        rows.iter().all(|row| row["read_at"].is_null()),
        "{}",
        serde_json::to_string(rows).expect("rows")
    );
    for row in rows {
        assert!(row["id"].is_string());
        assert!(row["agent_id"].is_string());
        assert!(row["created_at"].is_u64());
    }

    let unread = second
        .ask("notification.list", json!({"unread_only": true}))
        .await;
    assert_eq!(unread["result"].as_array().expect("rows").len(), 3);
}

#[tokio::test]
async fn a_read_marks_one_row_and_read_agent_marks_one_agents_rows() {
    let office = Office::open().await;
    let mut maker = office.client().await;
    let ana = maker.create_agent(&office, "Ana", "false", None).await;
    let raj = maker.create_agent(&office, "Raj", "false", None).await;
    drop(maker);

    let parse = |id: &str| id.parse::<AgentId>().expect("agent id");
    events_with_no_client(
        &office,
        &[
            (parse(&ana), AgentStatus::Exited),
            (parse(&raj), AgentStatus::Failed),
        ],
    );

    let mut client = office.client().await;
    let listed = loop {
        let answer = client.ask("notification.list", json!({})).await;
        if answer["result"]
            .as_array()
            .is_some_and(|rows| rows.len() == 2)
        {
            break answer;
        }
        tokio::time::sleep(POLL).await;
    };
    let ana_row = listed["result"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|row| row["agent_name"] == "Ana")
        .expect("the Ana row")
        .clone();

    let read = client
        .ask(
            "notification.read",
            json!({"notification_id": ana_row["id"]}),
        )
        .await;
    assert_eq!(read["ok"], true, "{read}");
    let again = client
        .ask(
            "notification.read",
            json!({"notification_id": ana_row["id"]}),
        )
        .await;
    assert_eq!(again["ok"], true, "an already-read row is left as it is");

    let unread = client
        .ask("notification.list", json!({"unread_only": true}))
        .await;
    assert_eq!(unread_of(&unread["result"]), ["Raj"]);

    let by_agent = client
        .ask("notification.read_agent", json!({"agent_id": raj}))
        .await;
    assert_eq!(by_agent["ok"], true, "{by_agent}");
    let empty = client
        .ask("notification.list", json!({"unread_only": true}))
        .await;
    assert_eq!(empty["result"].as_array().expect("rows").len(), 0);

    // An unknown agent id is not an error: the rows are the target.
    let unknown = client
        .ask("notification.read_agent", json!({"agent_id": ana}))
        .await;
    assert_eq!(unknown["ok"], true, "{unknown}");

    let missing = client
        .ask(
            "notification.read",
            json!({"notification_id": "00000000-0000-0000-0000-00000000000f"}),
        )
        .await;
    let (code, message) = error_text(&missing);
    assert_eq!(code, "not_found");
    assert!(message.contains("was not found"), "{message}");

    // A bad body names the field.
    let bad = client
        .ask("notification.read", json!({"notification_id": 5}))
        .await;
    let (code, message) = error_text(&bad);
    assert_eq!(code, "invalid");
    assert!(message.contains("notification_id"), "{message}");
}

#[tokio::test]
async fn a_new_event_reaches_a_connected_client_as_notification_created() {
    let office = Office::open().await;
    let mut maker = office.client().await;
    let ana = maker.create_agent(&office, "Ana", "false", None).await;
    drop(maker);
    let ana_id = ana.parse::<AgentId>().expect("agent id");

    // The first client attaches before the event, so the event arrives live.
    let mut client = office.client().await;
    office
        .serving
        .office
        .services
        .events
        .publish(&agent_status_event(ana_id, AgentStatus::Exited, None));

    let event = client.next_event("notification.created").await;
    assert_eq!(event["notification"]["event"], "exited");
    assert_eq!(event["notification"]["agent_name"], "Ana");
    assert_eq!(event["notification"]["agent_id"], ana.as_str());
    assert!(event["notification"]["read_at"].is_null());

    // The row is already in the list the same client reads.
    let listed = client.ask("notification.list", json!({})).await;
    assert_eq!(names_of(&listed["result"]), ["Ana"]);
}
