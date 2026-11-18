// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]
#![cfg(unix)]

//! T6 at protocol level: the `agent.*`, `project.*`, `settings.*`, and
//! `agent.actions` requests and their events through a real socket, with fake
//! CLIs started through the whole server (the bridges, the launcher, and the
//! PTY runtime).

mod support;

use std::path::Path;
use std::time::{Duration, Instant};

use kernel::AgentId;

use office_harnessd::server::ServerOptions;
use serde_json::{Value, json};
use support::fake_cli::FakeClis;
use support::running_office::{ServingOffice, serve_office_with};
use support::wire_client::WireClient;

const QUIET_PERIOD_MS: u64 = 400;
const SNAPSHOT_HEADER_BYTES: usize = 17;
const MARKER_WAIT: Duration = Duration::from_secs(10);

struct Office {
    serving: ServingOffice,
    clis: FakeClis,
    work_folder: tempfile::TempDir,
}

impl Office {
    async fn open() -> Self {
        Self::open_with(FakeClis::new()).await
    }

    async fn open_with(clis: FakeClis) -> Self {
        let serving = serve_office_with(
            ServerOptions::default(),
            clis.login_setup(),
            Some(QUIET_PERIOD_MS),
        )
        .await;
        Self {
            serving,
            clis,
            work_folder: tempfile::tempdir().expect("work folder"),
        }
    }

    async fn client(&self) -> WireClient {
        WireClient::authenticate_ok(self.serving.office.port, &self.serving.office.token()).await
    }

    fn work_path(&self) -> &str {
        self.work_folder.path().to_str().expect("utf-8 path")
    }
}

/// Sends a request and returns its answer, skipping events that arrive first.
async fn call(client: &mut WireClient, id: u32, kind: &str, fields: Value) -> Value {
    let mut body = fields;
    body["id"] = json!(id);
    body["type"] = json!(kind);
    client.send_text(&body.to_string()).await;
    loop {
        let message = client.receive_json().await;
        if message["id"] == json!(id) {
            return message;
        }
    }
}

fn error_text(answer: &Value) -> (&str, &str) {
    (
        answer["error"]["code"].as_str().expect("code"),
        answer["error"]["message"].as_str().expect("message"),
    )
}

async fn create_agent(
    office: &Office,
    client: &mut WireClient,
    name: &str,
    command: &str,
) -> String {
    let answer = call(
        client,
        1,
        "agent.create",
        json!({"name": name, "command": command, "working_folder": office.work_path()}),
    )
    .await;
    assert_eq!(answer["ok"], true, "{answer}");
    answer["result"]["id"].as_str().expect("id").to_owned()
}

/// The agent an event is about, whichever field names it.
fn event_agent(message: &Value) -> &Value {
    [
        &message["agent_id"],
        &message["entry"]["agent_id"],
        &message["agent"]["id"],
    ]
    .into_iter()
    .find(|field| !field.is_null())
    .unwrap_or(&Value::Null)
}

/// Reads events until one of the type `kind` names the agent. Returns it.
async fn next_event(client: &mut WireClient, kind: &str, agent_id: &str) -> Value {
    loop {
        let message = client.receive_json().await;
        if message["type"] == kind && event_agent(&message) == agent_id {
            return message;
        }
    }
}

/// The statuses of each agent, in order, until every agent in `finals` has
/// reached its last status.
async fn statuses_until(
    client: &mut WireClient,
    finals: &[(&str, &str)],
) -> std::collections::HashMap<String, Vec<String>> {
    let mut seen: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    let done = |seen: &std::collections::HashMap<String, Vec<String>>| {
        finals.iter().all(|(agent, last)| {
            seen.get(*agent)
                .and_then(|list| list.last())
                .is_some_and(|status| status == last)
        })
    };
    while !done(&seen) {
        let message = client.receive_json().await;
        if message["type"] == "agent.status" {
            let agent = message["agent_id"].as_str().expect("agent").to_owned();
            let status = message["status"].as_str().expect("status").to_owned();
            seen.entry(agent).or_default().push(status);
        }
    }
    seen
}

async fn status_of(client: &mut WireClient, agent_id: &str) -> String {
    let listed = call(client, 90, "agent.list", json!({})).await;
    listed["result"]
        .as_array()
        .expect("agents")
        .iter()
        .find(|agent| agent["id"] == agent_id)
        .and_then(|agent| agent["status"].as_str())
        .expect("the agent is listed")
        .to_owned()
}

fn snapshot_text(frame: &[u8]) -> String {
    String::from_utf8_lossy(&frame[SNAPSHOT_HEADER_BYTES..]).into_owned()
}

/// The snapshot may come before the agent prints its marker, so this reads
/// the live frames until `own` shows or `MARKER_WAIT` passes. `other` must
/// appear nowhere in what it read.
async fn read_until_marker(client: &mut WireClient, own: &str, other: &str) -> String {
    let mut text = snapshot_text(&client.receive_binary().await);
    let deadline = Instant::now() + MARKER_WAIT;
    loop {
        assert!(!text.contains(other), "{text}");
        if text.contains(own) {
            return text;
        }
        assert!(Instant::now() < deadline, "no {own} in time: {text}");
        text.push_str(&String::from_utf8_lossy(&client.receive_binary().await));
    }
}

#[tokio::test]
async fn projects_are_created_listed_and_deleted_and_bad_requests_name_the_field() {
    let office = Office::open().await;
    let mut client = office.client().await;

    let created = call(
        &mut client,
        1,
        "project.create",
        json!({"name": "Docs", "folder": office.work_path()}),
    )
    .await;
    assert_eq!(created["ok"], true, "{created}");
    assert_eq!(created["result"]["name"], "Docs");
    assert_eq!(created["result"]["folder"], office.work_path());
    let project_id = created["result"]["id"].as_str().expect("id").to_owned();

    let listed = call(&mut client, 2, "project.list", json!({})).await;
    assert_eq!(listed["result"].as_array().expect("projects").len(), 1);

    let blank = call(
        &mut client,
        3,
        "project.create",
        json!({"name": " ", "folder": office.work_path()}),
    )
    .await;
    assert_eq!(error_text(&blank).0, "invalid");
    assert!(error_text(&blank).1.contains("project name"));

    let relative = call(
        &mut client,
        4,
        "project.create",
        json!({"name": "Rel", "folder": "docs"}),
    )
    .await;
    assert_eq!(error_text(&relative).0, "invalid");
    assert!(error_text(&relative).1.contains("project folder"));

    let missing = call(&mut client, 5, "project.create", json!({"name": "Rel"})).await;
    assert_eq!(error_text(&missing).0, "invalid");
    assert!(error_text(&missing).1.contains("folder"));

    let duplicate = call(
        &mut client,
        6,
        "project.create",
        json!({"name": "dOCS", "folder": office.work_path()}),
    )
    .await;
    assert_eq!(error_text(&duplicate).0, "conflict");
    assert!(error_text(&duplicate).1.contains("project name"));

    let unknown = call(
        &mut client,
        7,
        "project.delete",
        json!({"project_id": "00000000-0000-4000-8000-000000000000"}),
    )
    .await;
    assert_eq!(error_text(&unknown).0, "not_found");

    let bad_id = call(
        &mut client,
        8,
        "project.delete",
        json!({"project_id": "nope"}),
    )
    .await;
    assert_eq!(error_text(&bad_id).0, "invalid");
    assert!(error_text(&bad_id).1.contains("project_id"));

    let deleted = call(
        &mut client,
        9,
        "project.delete",
        json!({"project_id": project_id}),
    )
    .await;
    assert_eq!(deleted["ok"], true);
    assert_eq!(deleted["result"], Value::Null);
    let after = call(&mut client, 10, "project.list", json!({})).await;
    assert_eq!(after["result"].as_array().expect("projects").len(), 0);
    office.serving.stop().await;
}

#[tokio::test]
async fn agents_are_created_and_listed_and_bad_requests_name_the_field() {
    let office = Office::open().await;
    let mut client = office.client().await;

    let created = call(&mut client, 1, "agent.create", json!({"name": "Ana", "command": "claude --model opus", "working_folder": office.work_path()})).await;
    assert_eq!(created["ok"], true, "{created}");
    assert_eq!(created["result"]["status"], "stopped");
    assert_eq!(created["result"]["command"], "claude --model opus");

    let listed = call(&mut client, 2, "agent.list", json!({})).await;
    assert_eq!(listed["result"][0]["name"], "Ana");

    let cases = [
        (
            json!({"name": "", "command": "x", "working_folder": office.work_path()}),
            "invalid",
            "agent name",
        ),
        (
            json!({"name": "Ben", "command": "  ", "working_folder": office.work_path()}),
            "invalid",
            "command",
        ),
        (
            json!({"name": "Ben", "command": "x"}),
            "invalid",
            "working folder or a project",
        ),
        (
            json!({"name": "Ben", "command": "x", "working_folder": "relative"}),
            "invalid",
            "working folder",
        ),
        (
            json!({"name": "Ben", "command": "x \"open", "working_folder": office.work_path()}),
            "invalid",
            "command",
        ),
        (
            json!({"name": "ANA", "command": "x", "working_folder": office.work_path()}),
            "conflict",
            "agent name",
        ),
        (
            json!({"name": "Ben", "working_folder": office.work_path()}),
            "invalid",
            "command",
        ),
        (
            json!({"name": 5, "command": "x", "working_folder": office.work_path()}),
            "invalid",
            "name",
        ),
        (
            json!({"name": "Ben", "command": "x", "working_folder": 7}),
            "invalid",
            "working_folder",
        ),
        (
            json!({"name": "Ben", "command": "x", "project_id": "00000000-0000-4000-8000-000000000000"}),
            "not_found",
            "project",
        ),
        (
            json!({"name": "Ben", "command": "x", "project_id": "nope"}),
            "invalid",
            "project_id",
        ),
    ];
    for (offset, (fields, code, names)) in cases.into_iter().enumerate() {
        let id = 10 + u32::try_from(offset).expect("small");
        let answer = call(&mut client, id, "agent.create", fields).await;
        assert_eq!(error_text(&answer).0, code, "{answer}");
        assert!(error_text(&answer).1.contains(names), "{answer}");
    }

    let unknown = call(
        &mut client,
        30,
        "agent.start",
        json!({"agent_id": "00000000-0000-4000-8000-000000000000"}),
    )
    .await;
    assert_eq!(error_text(&unknown).0, "not_found");
    let unsupported = call(&mut client, 31, "agent.nonesuch", json!({})).await;
    assert_eq!(error_text(&unsupported).0, "not_found");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_project_in_use_cannot_be_deleted_and_the_refusal_names_its_agents() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let project = call(
        &mut client,
        1,
        "project.create",
        json!({"name": "Docs", "folder": office.work_path()}),
    )
    .await;
    let project_id = project["result"]["id"].as_str().expect("id").to_owned();
    let created = call(
        &mut client,
        2,
        "agent.create",
        json!({"name": "Ana", "command": "x", "project_id": project_id}),
    )
    .await;
    assert_eq!(created["ok"], true, "{created}");

    let refused = call(
        &mut client,
        3,
        "project.delete",
        json!({"project_id": project_id}),
    )
    .await;

    assert_eq!(error_text(&refused).0, "conflict");
    assert!(error_text(&refused).1.contains("Ana"));
    office.serving.stop().await;
}

#[tokio::test]
async fn settings_are_read_and_changed_and_bad_ones_name_the_setting() {
    let office = Office::open().await;
    let mut client = office.client().await;

    let read = call(
        &mut client,
        1,
        "settings.get",
        json!({"key": "stop_grace_ms"}),
    )
    .await;
    assert_eq!(
        read["result"],
        json!({"key": "stop_grace_ms", "value": "10000"})
    );

    let set = call(
        &mut client,
        2,
        "settings.set",
        json!({"key": "stop_grace_ms", "value": "2500"}),
    )
    .await;
    assert_eq!(
        set["result"],
        json!({"key": "stop_grace_ms", "value": "2500"})
    );
    let again = call(
        &mut client,
        3,
        "settings.get",
        json!({"key": "stop_grace_ms"}),
    )
    .await;
    assert_eq!(again["result"]["value"], "2500");

    let bad_value = call(
        &mut client,
        4,
        "settings.set",
        json!({"key": "stop_grace_ms", "value": "soon"}),
    )
    .await;
    assert_eq!(error_text(&bad_value).0, "invalid");
    assert!(error_text(&bad_value).1.contains("stop_grace_ms"));
    let unknown = call(&mut client, 5, "settings.get", json!({"key": "volume"})).await;
    assert_eq!(error_text(&unknown).0, "invalid");
    assert!(error_text(&unknown).1.contains("volume"));
    let no_key = call(&mut client, 6, "settings.set", json!({"value": "1"})).await;
    assert!(error_text(&no_key).1.contains("key"));
    office.serving.stop().await;
}

#[tokio::test]
async fn the_last_view_is_stored_and_read_back() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let value = json!({"project": "p-1", "agent": null, "terminal_open": true}).to_string();

    let set = call(
        &mut client,
        1,
        "settings.set",
        json!({"key": "last_view", "value": value}),
    )
    .await;
    assert_eq!(set["result"], json!({"key": "last_view", "value": value}));

    let read = call(&mut client, 2, "settings.get", json!({"key": "last_view"})).await;
    assert_eq!(read["result"]["value"], value);
    office.serving.stop().await;
}

#[tokio::test]
async fn a_last_view_that_is_oversize_or_not_the_shape_is_refused() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let long_id = "p".repeat(1_024);

    let oversize = call(
        &mut client,
        1,
        "settings.set",
        json!({"key": "last_view", "value": json!({"project": long_id, "agent": null, "terminal_open": false}).to_string()}),
    )
    .await;
    assert_eq!(error_text(&oversize).0, "invalid");
    assert!(error_text(&oversize).1.contains("last_view"));

    let wrong_shape = call(
        &mut client,
        2,
        "settings.set",
        json!({"key": "last_view", "value": r#"{"project": null}"#}),
    )
    .await;
    assert_eq!(error_text(&wrong_shape).0, "invalid");
    assert!(error_text(&wrong_shape).1.contains("last_view"));

    let kept = call(&mut client, 3, "settings.get", json!({"key": "last_view"})).await;
    assert_eq!(kept["result"]["value"], "");
    office.serving.stop().await;
}

#[tokio::test]
async fn every_fixed_shortcut_refuses_the_focus_shortcut_and_names_the_conflict() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let reserved = [
        ("Ctrl+Shift+F1", "agent 1"),
        ("Ctrl+Shift+F2", "agent 2"),
        ("Ctrl+Shift+F3", "agent 3"),
        ("Ctrl+Shift+F4", "agent 4"),
        ("Ctrl+Shift+F5", "agent 5"),
        ("Ctrl+Shift+F6", "agent 6"),
        ("Ctrl+Shift+F7", "agent 7"),
        ("Ctrl+Shift+F8", "agent 8"),
        ("Ctrl+Shift+F9", "agent 9"),
        ("Ctrl+Shift+F10", "the agent switcher"),
    ];
    for (number, (shortcut, reserved_for)) in reserved.iter().enumerate() {
        let id = u32::try_from(number).expect("request id") + 1;
        let answer = call(
            &mut client,
            id,
            "settings.set",
            json!({"key": "focus_shortcut", "value": shortcut}),
        )
        .await;
        let (code, message) = error_text(&answer);
        assert_eq!(code, "conflict", "{shortcut}");
        assert!(message.contains(reserved_for), "{shortcut}: {message}");
        assert!(message.contains(shortcut), "{shortcut}: {message}");
    }
    office.serving.stop().await;
}

#[tokio::test]
async fn a_missing_working_folder_refuses_the_start_and_names_it() {
    let office = Office::open().await;
    let mut client = office.client().await;
    office.clis.add("fake-cli", "exit 0");
    let missing = Path::new(office.work_path()).join("no-such-folder");
    let created = call(&mut client, 1, "agent.create", json!({"name": "Ana", "command": "fake-cli", "working_folder": missing.to_str().expect("utf-8")})).await;
    let agent_id = created["result"]["id"].as_str().expect("id").to_owned();

    let refused = call(&mut client, 2, "agent.start", json!({"agent_id": agent_id})).await;

    assert_eq!(error_text(&refused).0, "invalid");
    assert!(error_text(&refused).1.contains("no-such-folder"));
    assert_eq!(status_of(&mut client, &agent_id).await, "stopped");
    office.serving.stop().await;
}

#[tokio::test]
async fn each_action_adds_one_entry_for_the_user_and_agent_actions_returns_it() {
    let office = Office::open().await;
    office.clis.add("fake-cli", "echo hello; sleep 30");
    let mut client = office.client().await;
    let mut observer = office.client().await;
    let agent_id = create_agent(&office, &mut client, "Ana", "fake-cli").await;
    let created = next_event(&mut observer, "action.recorded", &agent_id).await;
    assert_eq!(created["entry"]["action"], "create");

    let started = call(&mut client, 2, "agent.start", json!({"agent_id": agent_id})).await;
    assert_eq!(started["ok"], true, "{started}");
    let stopped = call(&mut client, 3, "agent.stop", json!({"agent_id": agent_id})).await;
    assert_eq!(stopped["ok"], true, "{stopped}");
    let deleted = call(
        &mut client,
        4,
        "agent.delete",
        json!({"agent_id": agent_id}),
    )
    .await;
    assert_eq!(deleted["ok"], true, "{deleted}");

    let actions = call(
        &mut client,
        5,
        "agent.actions",
        json!({"agent_id": agent_id}),
    )
    .await;
    let entries = actions["result"].as_array().expect("entries");
    let names: Vec<&str> = entries
        .iter()
        .map(|entry| entry["action"].as_str().expect("action"))
        .collect();
    assert_eq!(names, ["create", "start", "stop", "delete"], "{actions}");
    for entry in entries {
        assert_eq!(entry["actor"], "user");
        assert_eq!(entry["agent_id"], agent_id);
        assert_eq!(entry["result"], "ok");
        assert!(entry["at"].as_i64().expect("time") > 0);
    }
    let started_entry = next_event(&mut observer, "action.recorded", &agent_id).await;
    assert_eq!(started_entry["entry"]["action"], "start");

    let bad = call(&mut client, 6, "agent.actions", json!({"agent_id": "nope"})).await;
    assert_eq!(error_text(&bad).0, "invalid");
    assert!(error_text(&bad).1.contains("agent_id"));
    office.serving.stop().await;
}

#[tokio::test]
async fn deleting_an_agent_sends_agent_deleted_to_a_second_client() {
    let office = Office::open().await;
    office.clis.add("fake-cli", "echo up; sleep 30");
    let mut client = office.client().await;
    let mut attached = office.client().await;
    let agent_id = create_agent(&office, &mut client, "Ana", "fake-cli").await;
    let changed = next_event(&mut attached, "agent.changed", &agent_id).await;
    assert_eq!(changed["agent"]["name"], "Ana");
    let started = call(&mut client, 2, "agent.start", json!({"agent_id": agent_id})).await;
    assert_eq!(started["ok"], true, "{started}");
    let attach = json!({"agent_id": agent_id, "cols": 80, "rows": 24});
    let answer = call(&mut attached, 3, "terminal.attach", attach).await;
    assert_eq!(answer["ok"], true, "{answer}");

    let deleted = call(
        &mut client,
        4,
        "agent.delete",
        json!({"agent_id": agent_id}),
    )
    .await;
    assert_eq!(deleted["ok"], true);

    let event = next_event(&mut attached, "agent.deleted", &agent_id).await;
    assert_eq!(event["agent_id"], agent_id);
    let listed = call(&mut client, 5, "agent.list", json!({})).await;
    assert_eq!(listed["result"].as_array().expect("agents").len(), 0);
    office.serving.stop().await;
}

#[tokio::test]
async fn a_command_that_cannot_start_fails_that_agent_only() {
    let office = Office::open().await;
    office.clis.add("fake-cli", "echo up; sleep 30");
    let mut client = office.client().await;
    let healthy = create_agent(&office, &mut client, "Healthy", "fake-cli").await;
    let broken = create_agent(&office, &mut client, "Broken", "no-such-cli").await;
    let started = call(&mut client, 2, "agent.start", json!({"agent_id": healthy})).await;
    assert_eq!(started["ok"], true, "{started}");

    let failed = call(&mut client, 3, "agent.start", json!({"agent_id": broken})).await;

    assert_eq!(failed["ok"], true, "{failed}");
    assert!(
        failed["result"]["exit_result"]
            .as_str()
            .expect("exit result")
            .starts_with("spawn-error: ")
    );
    assert!(
        failed["result"]["exit_result"]
            .as_str()
            .expect("text")
            .contains("no-such-cli")
    );
    assert_eq!(failed["result"]["can_continue"], false);
    assert_eq!(status_of(&mut client, &broken).await, "failed");
    assert_ne!(status_of(&mut client, &healthy).await, "failed");
    let actions = call(&mut client, 4, "agent.actions", json!({"agent_id": broken})).await;
    assert!(
        actions["result"][1]["result"]
            .as_str()
            .expect("result")
            .contains("no-such-cli")
    );
    let stopped = call(&mut client, 5, "agent.stop", json!({"agent_id": healthy})).await;
    assert_eq!(stopped["ok"], true, "{stopped}");
    office.serving.stop().await;
}

#[tokio::test]
async fn two_agents_run_side_by_side_without_interfering() {
    let office = Office::open().await;
    office.clis.add("first-cli", "echo marker-first; sleep 30");
    office
        .clis
        .add("second-cli", "echo marker-second; sleep 30");
    let mut client = office.client().await;
    let first = create_agent(&office, &mut client, "First", "first-cli").await;
    let second = create_agent(&office, &mut client, "Second", "second-cli").await;
    for (id, agent) in [(2, &first), (3, &second)] {
        let started = call(&mut client, id, "agent.start", json!({"agent_id": agent})).await;
        assert_eq!(started["ok"], true, "{started}");
    }
    let mut observer = office.client().await;

    // Each agent shows its own output only, and stopping one leaves the other.
    for (id, agent, own, other) in [
        (4, &first, "marker-first", "marker-second"),
        (5, &second, "marker-second", "marker-first"),
    ] {
        let attached = call(
            &mut observer,
            id,
            "terminal.attach",
            json!({"agent_id": agent, "cols": 80, "rows": 24}),
        )
        .await;
        assert_eq!(attached["ok"], true, "{attached}");
        read_until_marker(&mut observer, own, other).await;
    }
    let stopped = call(&mut client, 6, "agent.stop", json!({"agent_id": first})).await;
    assert_eq!(stopped["ok"], true, "{stopped}");
    assert_eq!(status_of(&mut client, &first).await, "stopped");
    assert!(
        ["working", "waiting_for_input"].contains(&status_of(&mut client, &second).await.as_str())
    );
    let stopped = call(&mut client, 7, "agent.stop", json!({"agent_id": second})).await;
    assert_eq!(stopped["ok"], true, "{stopped}");
    office.serving.stop().await;
}

#[tokio::test]
async fn the_status_follows_starting_working_waiting_and_exited_or_failed() {
    let office = Office::open().await;
    office
        .clis
        .add("quiet-cli", "echo ready; sleep 2; echo again; sleep 0.1");
    office.clis.add("failing-cli", "echo oops; sleep 2; exit 3");
    let mut client = office.client().await;
    let mut observer = office.client().await;
    let finishing = create_agent(&office, &mut client, "Finishing", "quiet-cli").await;
    let failing = create_agent(&office, &mut client, "Failing", "failing-cli").await;

    for (id, agent) in [(2, &finishing), (3, &failing)] {
        let started = call(&mut client, id, "agent.start", json!({"agent_id": agent})).await;
        assert_eq!(started["ok"], true, "{started}");
        assert_eq!(started["result"]["ended_at"], Value::Null, "{started}");
    }

    let finals = [(finishing.as_str(), "exited"), (failing.as_str(), "failed")];
    let seen = statuses_until(&mut observer, &finals).await;
    assert_eq!(
        seen[&finishing],
        [
            "starting",
            "working",
            "waiting_for_input",
            "working",
            "exited"
        ]
    );
    assert_eq!(
        seen[&failing],
        ["starting", "working", "waiting_for_input", "failed"]
    );
    office.serving.stop().await;
}

#[tokio::test]
async fn a_stopped_agent_shows_stopped_and_can_start_again() {
    let office = Office::open().await;
    office.clis.add("fake-cli", "echo hi; sleep 30");
    let mut client = office.client().await;
    let agent_id = create_agent(&office, &mut client, "Ana", "fake-cli").await;
    call(&mut client, 2, "agent.start", json!({"agent_id": agent_id})).await;

    let running_start = call(&mut client, 3, "agent.start", json!({"agent_id": agent_id})).await;
    assert_eq!(error_text(&running_start).0, "conflict");
    let stopped = call(&mut client, 4, "agent.stop", json!({"agent_id": agent_id})).await;
    assert_eq!(stopped["ok"], true, "{stopped}");
    assert_eq!(status_of(&mut client, &agent_id).await, "stopped");
    let idle_stop = call(&mut client, 5, "agent.stop", json!({"agent_id": agent_id})).await;
    assert_eq!(error_text(&idle_stop).0, "conflict");

    let again = call(&mut client, 6, "agent.start", json!({"agent_id": agent_id})).await;
    assert_eq!(again["ok"], true, "{again}");
    assert_eq!(again["result"]["ended_at"], Value::Null, "{again}");
    call(&mut client, 7, "agent.stop", json!({"agent_id": agent_id})).await;
    office.serving.stop().await;
}

#[tokio::test]
async fn a_cli_only_the_login_path_knows_runs_with_a_clean_environment() {
    let office = Office::open().await;
    office.clis.add(
        "fake-cli",
        "echo \"appdir=[$APPDIR] term=[$TERM] oldpwd=[$OLDPWD]\"",
    );
    let mut client = office.client().await;
    let agent_id = create_agent(&office, &mut client, "Ana", "fake-cli").await;
    let mut observer = office.client().await;

    let started = call(&mut client, 2, "agent.start", json!({"agent_id": agent_id})).await;
    assert_eq!(started["ok"], true, "{started}");
    assert_eq!(started["result"]["ended_at"], Value::Null, "{started}");
    let seen = statuses_until(&mut observer, &[(&agent_id, "exited")]).await;
    assert_eq!(seen[&agent_id].last().map(String::as_str), Some("exited"));

    let attached = call(
        &mut client,
        3,
        "terminal.attach",
        json!({"agent_id": agent_id, "cols": 100, "rows": 30}),
    )
    .await;
    assert_eq!(attached["ok"], true, "{attached}");
    let text = snapshot_text(&client.receive_binary().await);
    assert!(text.contains("appdir=[]"), "{text}");
    assert!(text.contains("term=[xterm-256color]"), "{text}");
    assert!(text.contains("oldpwd=[]"), "{text}");
    office.serving.stop().await;
}

/// Reads messages until the answer to `id`, and returns it with every message
/// that came before it.
async fn call_collecting(
    client: &mut WireClient,
    id: u32,
    kind: &str,
    fields: Value,
) -> (Value, Vec<Value>) {
    let mut body = fields;
    body["id"] = json!(id);
    body["type"] = json!(kind);
    client.send_text(&body.to_string()).await;
    let mut before = Vec::new();
    loop {
        let message = client.receive_json().await;
        if message["id"] == json!(id) {
            return (message, before);
        }
        before.push(message);
    }
}

#[tokio::test]
async fn a_delete_during_a_slow_start_waits_for_it_and_leaves_no_child_running() {
    let office = Office::open_with(FakeClis::new().with_slow_login(1)).await;
    office.clis.add("fake-cli", "echo up; sleep 30");
    let mut first_client = office.client().await;
    let mut second_client = office.client().await;
    let agent_id = create_agent(&office, &mut first_client, "Ana", "fake-cli").await;
    first_client
        .send_text(&json!({"id": 2, "type": "agent.start", "agent_id": agent_id}).to_string())
        .await;
    // The start is inside the login shell now.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let delete_answer = call(
        &mut second_client,
        3,
        "agent.delete",
        json!({"agent_id": agent_id}),
    )
    .await;

    assert_eq!(delete_answer["ok"], true, "{delete_answer}");
    let start_answer = loop {
        let message = first_client.receive_json().await;
        if message["id"] == json!(2) {
            break message;
        }
    };
    assert_eq!(start_answer["ok"], true, "{start_answer}");
    let runtime = &office.serving.office.services.terminal.runtime;
    let agent: AgentId = agent_id.parse().expect("id");
    assert!(
        !runtime.is_running(agent),
        "the child must not outlive its agent"
    );
    office.serving.stop().await;
}

#[tokio::test]
async fn a_stop_after_a_natural_exit_refuses_and_republishes_no_status() {
    let office = Office::open().await;
    office.clis.add("fake-cli", "echo hi");
    let mut client = office.client().await;
    let mut observer = office.client().await;
    let agent_id = create_agent(&office, &mut client, "Ana", "fake-cli").await;
    call(&mut client, 2, "agent.start", json!({"agent_id": agent_id})).await;
    statuses_until(&mut observer, &[(&agent_id, "exited")]).await;

    let (stopped, events) = call_collecting(
        &mut observer,
        3,
        "agent.stop",
        json!({"agent_id": agent_id}),
    )
    .await;

    assert_eq!(error_text(&stopped).0, "conflict");
    assert!(
        events.iter().all(|event| event["type"] != "agent.status"),
        "{events:?}"
    );
    assert_eq!(status_of(&mut client, &agent_id).await, "exited");
    office.serving.stop().await;
}

#[tokio::test]
async fn terminal_input_and_output_keep_flowing_while_a_slow_stop_runs() {
    let office = Office::open().await;
    office.clis.add(
        "stubborn-cli",
        "trap '' HUP TERM INT; echo up; while :; do sleep 1; done",
    );
    office.clis.add("echo-cli", "echo ready; cat");
    let mut client = office.client().await;
    let stubborn = create_agent(&office, &mut client, "Stubborn", "stubborn-cli").await;
    let chatty = create_agent(&office, &mut client, "Chatty", "echo-cli").await;
    call(
        &mut client,
        2,
        "settings.set",
        json!({"key": "stop_grace_ms", "value": "3000"}),
    )
    .await;
    for (id, agent) in [(3, &stubborn), (4, &chatty)] {
        let started = call(&mut client, id, "agent.start", json!({"agent_id": agent})).await;
        assert_eq!(started["ok"], true, "{started}");
    }
    let attached = call(
        &mut client,
        5,
        "terminal.attach",
        json!({"agent_id": chatty, "cols": 80, "rows": 24}),
    )
    .await;
    assert_eq!(attached["ok"], true, "{attached}");
    let began = Instant::now();
    client
        .send_text(&json!({"id": 6, "type": "agent.stop", "agent_id": stubborn}).to_string())
        .await;

    // "hello\n" in base64, typed while the stop waits out its grace period.
    let typed = call(
        &mut client,
        7,
        "terminal.input",
        json!({"agent_id": chatty, "data": "aGVsbG8K"}),
    )
    .await;
    assert_eq!(typed["ok"], true, "{typed}");
    let mut seen = String::new();
    while !seen.contains("hello") {
        seen.push_str(&snapshot_text(&client.receive_binary().await));
    }

    assert!(
        began.elapsed() < Duration::from_millis(2500),
        "the loop waited for the stop"
    );
    let stopped = loop {
        let message = client.receive_json().await;
        if message["id"] == json!(6) {
            break message;
        }
    };
    assert_eq!(stopped["ok"], true, "{stopped}");
    call(&mut client, 8, "agent.stop", json!({"agent_id": chatty})).await;
    office.serving.stop().await;
}

#[tokio::test]
async fn the_change_is_announced_before_its_record() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let mut observer = office.client().await;

    let agent_id = create_agent(&office, &mut client, "Ana", "fake-cli").await;

    let first = loop {
        let message = observer.receive_json().await;
        if message["type"] == "agent.changed" || message["type"] == "action.recorded" {
            break message;
        }
    };
    assert_eq!(first["type"], "agent.changed");
    let second = next_event(&mut observer, "action.recorded", &agent_id).await;
    assert_eq!(second["entry"]["action"], "create");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_failed_action_record_raises_storage_error_and_the_change_still_succeeds() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let mut observer = office.client().await;
    let database =
        rusqlite::Connection::open(office.serving.office.config.database_path()).expect("database");
    database
        .execute_batch("DROP TABLE audit_actions")
        .expect("drop");

    let created = call(
        &mut client,
        1,
        "agent.create",
        json!({"name": "Ana", "command": "x", "working_folder": office.work_path()}),
    )
    .await;

    assert_eq!(created["ok"], true, "{created}");
    let alert = loop {
        let message = observer.receive_json().await;
        if message["type"] == "storage.error" {
            break message;
        }
    };
    assert!(
        alert["message"]
            .as_str()
            .expect("message")
            .contains("action record")
    );
    assert!(office.serving.office.log_text().contains("action record"));
    office.serving.stop().await;
}
