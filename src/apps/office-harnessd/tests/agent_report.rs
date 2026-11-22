// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! `report.send` through the real helper (0015): a running agent reports to
//! the person through Needs you, bounded, in visible form, held and limited
//! like any agent-made action, and on the record.

mod support;

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;
use std::time::{Duration, Instant};

use agent_infrastructure::persistence::SharedConnection;
use hierarchy_application::commands::place_agent::{PlaceAgentHandler, PlaceAgentRequest};
use hierarchy_application::common::interfaces::HierarchyStore;
use hierarchy_domain::Role;
use hierarchy_infrastructure::persistence::repositories::hierarchy_repository::HierarchyRepository;
use hierarchy_infrastructure::services::agent_places::AgentPlaces;
use serde_json::{Value, json};
use support::office_client::{Client, Office, wait_for};

/// The real server binary that the fake CLIs run as the helper.
const HELPER: &str = env!("CARGO_BIN_EXE_office-harnessd");

const REPORT: &str = "report.send";

/// How long a report may take to show in Needs you (SC-003).
const SHOW_LIMIT: Duration = Duration::from_secs(2);

/// The longest report text (spec FR-001).
const REPORT_MAX_CHARS: usize = 1000;

/// The default reports limit per 60 seconds (spec FR-006).
const REPORT_LIMIT: usize = 5;

/// The characters that must never reach a reader as themselves (SC-004).
const RAW_CONTROLS: [char; 4] = ['\u{1b}', '\u{9b}', '\u{202e}', '\n'];

/// A fake CLI body that hands its session's environment to
/// `<records>/<name>.env`, runs `rest`, and stays alive.
fn sender_body(records: &Path, name: &str, rest: &str) -> String {
    let folder = records.display();
    format!(
        "env > {folder}/{name}.part\nmv {folder}/{name}.part {folder}/{name}.env\n{rest}\nsleep 30\n"
    )
}

/// The call credential and server file of the sender's live session.
struct Sender {
    credential: String,
    server_file: String,
}

async fn sender_of(records: &Path, name: &str) -> Sender {
    let path = records.join(format!("{name}.env"));
    let env = wait_for("the sender's environment", || {
        fs::read_to_string(&path).ok()
    })
    .await;
    let value = |variable: &str| {
        env.lines()
            .find_map(|line| line.strip_prefix(&format!("{variable}=")))
            .expect(variable)
            .to_owned()
    };
    Sender {
        credential: value("OFFICE_HARNESS_AGENT_TOKEN"),
        server_file: value("OFFICE_HARNESS_SERVER_FILE"),
    }
}

/// Runs the helper as the sender, on a blocking thread: waiting for it on
/// the runtime thread would stop the server it talks to.
async fn call(sender: &Sender, kind: &str, fields: &Value) -> Value {
    let kind = kind.to_owned();
    let credential = sender.credential.clone();
    let server_file = sender.server_file.clone();
    let fields = fields.to_string();
    let outcome: Output = tokio::task::spawn_blocking(move || {
        Command::new(HELPER)
            .args(["agent-call", &kind, &fields])
            .env("OFFICE_HARNESS_AGENT_TOKEN", credential)
            .env("OFFICE_HARNESS_SERVER_FILE", server_file)
            .output()
            .expect("helper")
    })
    .await
    .expect("helper task");
    serde_json::from_str(String::from_utf8_lossy(&outcome.stdout).trim()).expect("json")
}

async fn report(sender: &Sender, kind: &str, text: &str) -> Value {
    call(sender, REPORT, &json!({"kind": kind, "text": text})).await
}

/// Places an agent through the same store the server built: the wire has no
/// placement request, so the test writes the place directly.
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

/// Creates, places, and starts an agent whose CLI dumps its environment
/// under its name, runs `rest`, and stays alive.
async fn running_agent(
    office: &Office,
    client: &mut Client,
    name: &str,
    place_as: Option<(Role, Option<&str>)>,
    rest: &str,
) -> String {
    office
        .clis
        .add(name, &sender_body(office.records_path(), name, rest));
    let agent_id = client.create_agent(office, name, name, None).await;
    if let Some((role, parent)) = place_as {
        place(office, &agent_id, role, parent);
    }
    client.start(&agent_id).await;
    agent_id
}

async fn needs_you_items(client: &mut Client) -> Vec<Value> {
    let answer = client.ask("needs_you.list", json!({})).await;
    assert_eq!(answer["ok"], true, "{answer}");
    answer["result"]["items"].as_array().expect("items").clone()
}

fn item_of<'items>(items: &'items [Value], agent_id: &str) -> Option<&'items Value> {
    items.iter().find(|item| item["agent_id"] == agent_id)
}

async fn actions_of(client: &mut Client, agent_id: &str) -> Vec<Value> {
    let actions = client
        .ask("agent.actions", json!({"agent_id": agent_id}))
        .await;
    assert_eq!(actions["ok"], true, "{actions}");
    actions["result"].as_array().expect("entries").clone()
}

async fn report_notifications(client: &mut Client, agent_id: &str) -> usize {
    let answer = client.ask("notification.list", json!({})).await;
    assert_eq!(answer["ok"], true, "{answer}");
    answer["result"]
        .as_array()
        .expect("notifications")
        .iter()
        .filter(|row| row["agent_id"] == agent_id && row["event"] == "report")
        .count()
}

fn refusal_message(answer: &Value) -> String {
    assert_eq!(answer["ok"], false, "{answer}");
    answer["error"]["message"]
        .as_str()
        .expect("message")
        .to_owned()
}

fn holds_no_raw_control(text: &str) -> bool {
    !text
        .chars()
        .any(|character| RAW_CONTROLS.contains(&character))
}

#[tokio::test]
async fn the_ceo_reaches_needs_you_with_the_text_in_visible_form() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let ceo = running_agent(&office, &mut client, "ceo", Some((Role::Ceo, None)), "").await;
    let sender = sender_of(office.records_path(), "ceo").await;

    let started = Instant::now();
    let answer = report(
        &sender,
        "question",
        "pick\nA \u{1b}[31m or \u{9b}2J \u{202e}B",
    )
    .await;
    let items = needs_you_items(&mut client).await;

    assert_eq!(answer["ok"], true, "{answer}");
    assert_eq!(
        answer["result"],
        json!({"receiver": "person", "delivery": "needs_you"})
    );
    assert!(started.elapsed() < SHOW_LIMIT);
    let item = item_of(&items, &ceo).expect("the ceo's item");
    assert_eq!(item["kind"], "report", "{item}");
    assert_eq!(item["detail"], "question", "{item}");
    let shown = item["report"]["text"].as_str().expect("report text");
    assert_eq!(shown, "pick␊A ␛[31m or \\u009B2J \\u202EB");
    assert_eq!(report_notifications(&mut client, &ceo).await, 1);
    let record = actions_of(&mut client, &ceo).await;
    let entry = record
        .iter()
        .find(|entry| entry["action"] == "report")
        .expect("the report entry");
    assert_eq!(entry["actor"], json!(format!("agent:{ceo}")), "{entry}");
    let detail = entry["detail"].as_str().expect("detail");
    assert!(detail.starts_with("question: pick"), "{detail}");
    assert!(holds_no_raw_control(detail), "{detail}");
    office.serving.stop().await;
}

#[tokio::test]
async fn ten_reports_that_name_another_agent_are_refused_and_recorded() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let ceo = running_agent(&office, &mut client, "ceo", Some((Role::Ceo, None)), "").await;
    let lead = running_agent(
        &office,
        &mut client,
        "lead",
        Some((Role::Lead, Some(&ceo))),
        "",
    )
    .await;
    let sender = sender_of(office.records_path(), "lead").await;
    let before = actions_of(&mut client, &lead).await.len();

    for key in [
        "target_agent_id",
        "agent_id",
        "parent_agent_id",
        "team_id",
        "to",
    ] {
        for named in [&ceo, &lead] {
            let fields = json!({"kind": "done", "text": "x", key: named});
            let answer = call(&sender, REPORT, &fields).await;
            assert_eq!(answer["error"]["code"], "invalid", "{answer}");
        }
    }

    let refusals = actions_of(&mut client, &lead).await.len() - before;
    assert_eq!(refusals, 10);
    assert_eq!(needs_you_items(&mut client).await, Vec::<Value>::new());
    office.serving.stop().await;
}

#[tokio::test]
async fn a_report_at_the_length_bound_passes_and_longer_or_blank_ones_are_refused() {
    let office = Office::open().await;
    let mut client = office.client().await;
    running_agent(&office, &mut client, "ceo", Some((Role::Ceo, None)), "").await;
    let sender = sender_of(office.records_path(), "ceo").await;

    let at_bound = report(&sender, "info", &"x".repeat(REPORT_MAX_CHARS)).await;
    let over = report(&sender, "info", &"x".repeat(REPORT_MAX_CHARS + 1)).await;
    let blank = report(&sender, "info", "   ").await;

    assert_eq!(at_bound["ok"], true, "{at_bound}");
    assert_eq!(over["error"]["code"], "invalid", "{over}");
    assert_eq!(blank["error"]["code"], "invalid", "{blank}");
    office.serving.stop().await;
}

#[tokio::test]
async fn holds_and_the_reports_limit_refuse_and_the_third_limit_refusal_holds_the_sender() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let ceo = running_agent(&office, &mut client, "ceo", Some((Role::Ceo, None)), "").await;
    let sender = sender_of(office.records_path(), "ceo").await;

    let held = client.ask("hold.set", json!({})).await;
    assert_eq!(held["ok"], true, "{held}");
    for _ in 0..REPORT_LIMIT {
        let answer = report(&sender, "info", "held").await;
        assert!(refusal_message(&answer).contains("hold"), "{answer}");
    }
    let released = client.ask("hold.release", json!({})).await;
    assert_eq!(released["ok"], true, "{released}");

    for _ in 0..REPORT_LIMIT {
        let answer = report(&sender, "info", "step").await;
        assert_eq!(answer["ok"], true, "{answer}");
    }
    for _ in 0..3 {
        let answer = report(&sender, "info", "one more").await;
        assert!(
            refusal_message(&answer).contains("reports sent by one agent"),
            "{answer}"
        );
    }

    let holds = client.ask("hold.list", json!({})).await;
    let agents = holds["result"]["agents"].as_array().expect("agent holds");
    assert!(agents.iter().any(|hold| hold["agent_id"] == ceo), "{holds}");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_report_item_stays_through_work_and_a_wait_and_leaves_when_the_terminal_opens() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let records = office.records_path();
    let quiet_file = records.join("go-quiet");
    let chatty = format!(
        "while [ ! -e {} ]; do echo tick; sleep 0.1; done",
        quiet_file.display()
    );
    let ceo = running_agent(
        &office,
        &mut client,
        "ceo",
        Some((Role::Ceo, None)),
        &chatty,
    )
    .await;
    let member = running_agent(&office, &mut client, "member", None, "").await;
    client.wait_for_status(&ceo, "working").await;
    let ceo_sender = sender_of(records, "ceo").await;
    let member_sender = sender_of(records, "member").await;

    let answer = report(&ceo_sender, "question", "which branch?").await;
    assert_eq!(answer["ok"], true, "{answer}");
    let member_answer = report(&member_sender, "blocked", "no key").await;
    assert_eq!(member_answer["ok"], true, "{member_answer}");
    fs::write(&quiet_file, "").expect("quiet signal");
    client.wait_for_status(&ceo, "waiting_for_input").await;

    let items = needs_you_items(&mut client).await;
    let item = item_of(&items, &ceo).expect("the item stays");
    assert_eq!(item["report"]["text"], "which branch?", "{item}");

    let read = client
        .ask("notification.read_agent", json!({"agent_id": ceo}))
        .await;
    assert_eq!(read["ok"], true, "{read}");
    let dismissed = client
        .ask("needs_you.dismiss", json!({"agent_id": member}))
        .await;
    assert_eq!(dismissed["ok"], true, "{dismissed}");
    assert_eq!(needs_you_items(&mut client).await, Vec::<Value>::new());
    office.serving.stop().await;
}

#[tokio::test]
async fn five_reports_for_the_person_add_one_notification_and_show_the_fifth() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let ceo = running_agent(&office, &mut client, "ceo", Some((Role::Ceo, None)), "").await;
    let sender = sender_of(office.records_path(), "ceo").await;

    for step in 1..=REPORT_LIMIT {
        let answer = report(&sender, "info", &format!("step {step}")).await;
        assert_eq!(answer["ok"], true, "{answer}");
    }

    assert_eq!(report_notifications(&mut client, &ceo).await, 1);
    let items = needs_you_items(&mut client).await;
    let item = item_of(&items, &ceo).expect("the item");
    assert_eq!(item["report"]["text"], format!("step {REPORT_LIMIT}"));
    office.serving.stop().await;
}

#[tokio::test]
async fn a_member_marks_a_question_for_the_person_and_its_lead_gets_nothing() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let ceo = running_agent(&office, &mut client, "ceo", Some((Role::Ceo, None)), "").await;
    let lead = running_agent(
        &office,
        &mut client,
        "lead",
        Some((Role::Lead, Some(&ceo))),
        "",
    )
    .await;
    let member = running_agent(
        &office,
        &mut client,
        "member",
        Some((Role::Member, Some(&lead))),
        "",
    )
    .await;
    let sender = sender_of(office.records_path(), "member").await;

    let answer = call(
        &sender,
        REPORT,
        &json!({"kind": "question", "text": "may I push?", "for_person": true}),
    )
    .await;

    assert_eq!(answer["result"]["receiver"], "person", "{answer}");
    let items = needs_you_items(&mut client).await;
    assert!(item_of(&items, &member).is_some());
    let lead_record = actions_of(&mut client, &lead).await;
    assert!(
        !lead_record.iter().any(|entry| entry["action"] == "report"),
        "{lead_record:?}"
    );
    office.serving.stop().await;
}
