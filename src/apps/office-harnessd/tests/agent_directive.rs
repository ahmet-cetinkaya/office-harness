// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! A running agent sends a directive to its member over the agent-call
//! helper (0002, T5 acceptance d, e, f). The member's fake CLI records the
//! bytes the terminal typed, so the test reads the directive back as one
//! input followed by the Enter the server appended (SC-006).

mod support;

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;

use agent_infrastructure::persistence::SharedConnection;
use hierarchy_application::commands::place_agent::{PlaceAgentHandler, PlaceAgentRequest};
use hierarchy_application::common::interfaces::HierarchyStore;
use hierarchy_domain::Role;
use hierarchy_infrastructure::persistence::repositories::hierarchy_repository::HierarchyRepository;
use hierarchy_infrastructure::services::agent_places::AgentPlaces;
use kernel::AgentId;
use serde_json::{Value, json};
use support::office_client::{Client, Office, wait_for};

/// The real server binary that the fake CLIs run as the helper.
const HELPER: &str = env!("CARGO_BIN_EXE_office-harnessd");

/// The bytes one directive delivers: the text and the Enter (SC-006).
const DIRECTIVE: &str = "hello member";
const DIRECTIVE_BYTES: usize = DIRECTIVE.len() + 1;

/// A fake CLI that hands its environment to the records folder and stays
/// alive: the test then holds the credential of its live session.
fn env_cli(records: &Path) -> String {
    let folder = records.display();
    format!("env > {folder}/env.part\nmv {folder}/env.part {folder}/env\nsleep 30\n")
}

/// A fake CLI that hands its environment to the records folder, calls the
/// helper once with the given request, keeps the answer and exit code, and
/// stays alive for the test.
fn calling_cli(records: &Path, kind: &str, request: &Value) -> String {
    let folder = records.display();
    format!(
        "env > {folder}/env.part\nmv {folder}/env.part {folder}/env\n{HELPER} agent-call {kind} '{request}' > {folder}/call.out 2> {folder}/call.err\nprintf '%s' $? > {folder}/call.code\nsleep 30\n"
    )
}

/// Places an agent through the same store the server built: the wire has no
/// placement request yet, so the test writes the place directly, as
/// `agent_subtree.rs` does.
fn place(office: &Office, agent_id: &str, role: Role, parent: Option<&str>) {
    let connection =
        rusqlite::Connection::open(office.serving.office.config.database_path()).expect("database");
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

/// The action entries the wire holds for `agent_id`.
async fn actions_of(client: &mut Client, agent_id: &str) -> Vec<Value> {
    let actions = client
        .ask("agent.actions", json!({"agent_id": agent_id}))
        .await;
    assert_eq!(actions["ok"], true, "{actions}");
    actions["result"].as_array().expect("entries").clone()
}

/// The live credential of the first started CLI, from the environment the
/// server gave its session.
async fn live_credential(records: &Path) -> (String, String) {
    let env = wait_for("the session's environment", || {
        fs::read_to_string(records.join("env")).ok()
    })
    .await;
    let value = |name: &str| {
        env.lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .expect(name)
            .to_owned()
    };
    (
        value("OFFICE_HARNESS_AGENT_TOKEN"),
        value("OFFICE_HARNESS_SERVER_FILE"),
    )
}

/// Runs the helper with a credential, on a blocking thread: waiting for it
/// on the runtime thread would stop the server it talks to.
async fn send_directive(credential: &str, server_file: &str, request: &Value) -> Output {
    let credential = credential.to_owned();
    let server_file = server_file.to_owned();
    let request = request.to_string();
    tokio::task::spawn_blocking(move || {
        Command::new(HELPER)
            .args(["agent-call", "directive.send", &request])
            .env("OFFICE_HARNESS_AGENT_TOKEN", credential)
            .env("OFFICE_HARNESS_SERVER_FILE", server_file)
            .output()
            .expect("helper")
    })
    .await
    .expect("helper task")
}

/// The JSON answer of a failed helper call.
fn answer_of(outcome: &Output) -> Value {
    serde_json::from_str(String::from_utf8_lossy(&outcome.stdout).trim()).expect("json")
}

/// Builds the office, a CEO over a placed or unplaced subordinate, both
/// started, and returns the caller's credential. The CEO's CLI is the
/// environment dumper, so the test holds its live session's credential.
async fn office_with(office: &Office, member: Option<Role>) -> (String, String, String, String) {
    office.clis.add("member-cli", "echo ready; sleep 30");
    office.clis.add("lead-cli", &env_cli(office.records_path()));
    let mut client = office.client().await;
    let lead = client.create_agent(office, "Lead", "lead-cli", None).await;
    let member_id = client
        .create_agent(office, "Member", "member-cli", None)
        .await;
    place(office, &lead, Role::Ceo, None);
    if let Some(role) = member {
        place(office, &member_id, role, Some(&lead));
    }
    let started = client
        .ask("agent.start", json!({"agent_id": member_id}))
        .await;
    assert_eq!(started["ok"], true, "{started}");
    client.wait_for_status(&member_id, "working").await;
    let lead_started = client.ask("agent.start", json!({"agent_id": lead})).await;
    assert_eq!(lead_started["ok"], true, "{lead_started}");
    let (token, server_file) = live_credential(office.records_path()).await;
    (lead, member_id, token, server_file)
}

#[tokio::test]
async fn a_lead_sends_a_directive_and_the_member_takes_it_as_one_input_with_enter() {
    let office = Office::open().await;
    let typed = office.records_path().join("typed");
    // The member reads exactly the bytes of one directive: the text and the
    // Enter the server appends.
    office.clis.add(
        "member-cli",
        &format!(
            "echo ready\nstty raw -echo\ndd bs=1 count={DIRECTIVE_BYTES} of={} 2>/dev/null\nsleep 30",
            typed.display()
        ),
    );
    let mut client = office.client().await;
    let member_id = client
        .create_agent(&office, "Member", "member-cli", None)
        .await;
    // The lead calls the helper itself, with its own session credential.
    let request = json!({"target_agent_id": member_id, "text": DIRECTIVE});
    office.clis.add(
        "lead-cli",
        &calling_cli(office.records_path(), "directive.send", &request),
    );
    let lead = client.create_agent(&office, "Lead", "lead-cli", None).await;
    place(&office, &lead, Role::Ceo, None);
    place(&office, &member_id, Role::Lead, Some(&lead));
    let member_started = client
        .ask("agent.start", json!({"agent_id": member_id}))
        .await;
    assert_eq!(member_started["ok"], true, "{member_started}");
    client.wait_for_status(&member_id, "working").await;
    let lead_started = client.ask("agent.start", json!({"agent_id": lead})).await;
    assert_eq!(lead_started["ok"], true, "{lead_started}");

    // A redirect creates its file empty and fills it afterwards, so an
    // existing file is not a whole one: the wait needs content, not
    // existence.
    let code = wait_for("the helper's exit code", || {
        fs::read_to_string(office.records_path().join("call.code"))
            .ok()
            .filter(|text| !text.is_empty())
    })
    .await;
    let out = wait_for("the helper's answer", || {
        fs::read_to_string(office.records_path().join("call.out"))
            .ok()
            .filter(|text| !text.is_empty())
    })
    .await;
    assert_eq!(code, "0", "{out}");
    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    assert_eq!(answer["ok"], true, "{answer}");

    // SC-006: the text arrives as one input followed by Enter, nothing
    // between.
    let received = fs::read(&typed).expect("typed bytes");
    assert_eq!(received.len(), DIRECTIVE_BYTES, "{received:?}");
    assert_eq!(String::from_utf8_lossy(&received), format!("{DIRECTIVE}\r"));

    // The delivered attempt is in the record as the lead's directive on the
    // member (FR-011); the record is kept per target agent.
    let entries = actions_of(&mut client, &member_id).await;
    let entry = entries
        .iter()
        .find(|entry| entry["action"] == "directive")
        .expect("the directive is recorded");
    assert_eq!(entry["result"], "ok", "{entry}");
    assert_eq!(entry["actor"], json!(format!("agent:{lead}")), "{entry}");
    assert_eq!(entry["agent_id"], member_id, "{entry}");
    client.ask("agent.stop", json!({"agent_id": lead})).await;
    client
        .ask("agent.stop", json!({"agent_id": member_id}))
        .await;
    office.serving.stop().await;
}

#[tokio::test]
async fn a_directive_to_a_running_non_descendant_is_refused_and_recorded() {
    let office = Office::open().await;
    // The stranger stays unplaced, so it is outside the lead's subtree.
    let (lead, stranger, token, server_file) = office_with(&office, None).await;

    let request = json!({"target_agent_id": stranger, "text": "you there?"});
    let outcome = send_directive(&token, &server_file, &request).await;
    assert!(!outcome.status.success(), "{outcome:?}");
    let answer = answer_of(&outcome);
    assert_eq!(answer["error"]["code"], "refused", "{answer}");
    assert!(
        answer["error"]["message"]
            .as_str()
            .expect("message")
            .contains("not a descendant"),
        "{answer}"
    );

    // The refusal is recorded for the sender (FR-009), kept on the target's
    // entry like every action.
    let mut client = office.client().await;
    let entries = actions_of(&mut client, &stranger).await;
    let entry = entries
        .iter()
        .find(|entry| entry["action"] == "directive")
        .expect("the refused attempt is recorded");
    assert_ne!(entry["result"], "ok", "{entry}");
    assert_eq!(entry["actor"], json!(format!("agent:{lead}")), "{entry}");
    assert_eq!(entry["agent_id"], stranger, "{entry}");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_refused_directive_is_recorded_for_the_sender_and_for_the_target() {
    let office = Office::open().await;
    // The stranger stays unplaced, so the tree refuses the directive.
    let (lead, stranger, token, server_file) = office_with(&office, None).await;

    let request = json!({"target_agent_id": stranger, "text": "you there?"});
    let outcome = send_directive(&token, &server_file, &request).await;
    assert!(!outcome.status.success(), "{outcome:?}");

    // Both agents' records, read back, list the refusal once (ACF-168).
    let mut client = office.client().await;
    for agent in [&lead, &stranger] {
        let entries = actions_of(&mut client, agent).await;
        let refused = entries
            .iter()
            .filter(|entry| entry["action"] == "directive" && entry["result"] != "ok")
            .count();
        assert_eq!(refused, 1, "{agent}: {entries:?}");
    }
    office.serving.stop().await;
}

#[tokio::test]
async fn an_escape_in_the_text_is_invalid_and_recorded() {
    let office = Office::open().await;
    let (lead, member, token, server_file) = office_with(&office, Some(Role::Lead)).await;

    let request = json!({"target_agent_id": member, "text": "\u{1b}[D"});
    let outcome = send_directive(&token, &server_file, &request).await;
    assert!(!outcome.status.success(), "{outcome:?}");
    let answer = answer_of(&outcome);
    assert_eq!(answer["error"]["code"], "invalid", "{answer}");

    // The refused attempt is recorded with the text as it was sent (FR-011),
    // kept on the target's entry.
    let mut client = office.client().await;
    let entries = actions_of(&mut client, &member).await;
    let entry = entries
        .iter()
        .find(|entry| entry["action"] == "directive")
        .expect("the refused attempt is recorded");
    assert_ne!(entry["result"], "ok", "{entry}");
    assert_eq!(entry["actor"], json!(format!("agent:{lead}")), "{entry}");
    assert!(
        entry["result"]
            .as_str()
            .expect("result")
            .contains("control"),
        "{entry}"
    );
    office.serving.stop().await;
}

#[tokio::test]
async fn a_request_carrying_an_actor_field_is_refused_and_recorded() {
    let office = Office::open().await;
    let (lead, member, token, server_file) = office_with(&office, Some(Role::Lead)).await;

    let request = json!({
        "target_agent_id": member,
        "text": "hello",
        "actor": AgentId::generate().as_uuid_text(),
    });
    let outcome = send_directive(&token, &server_file, &request).await;
    assert!(!outcome.status.success(), "{outcome:?}");
    let answer = answer_of(&outcome);
    assert_eq!(answer["error"]["code"], "refused", "{answer}");
    assert!(
        answer["error"]["message"]
            .as_str()
            .expect("message")
            .contains("actor"),
        "{answer}"
    );

    // The gateway refusal is recorded too (FR-008).
    let mut client = office.client().await;
    let entries = actions_of(&mut client, &lead).await;
    let entry = entries
        .iter()
        .find(|entry| entry["action"] == "refused")
        .expect("the gateway refusal is recorded");
    assert_eq!(entry["actor"], json!(format!("agent:{lead}")), "{entry}");
    office.serving.stop().await;
}
