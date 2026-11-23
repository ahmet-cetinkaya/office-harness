// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! `agent.subtree` through the real helper (0014): a running agent reads the
//! state of its own descendants, and nothing else, with no terminal content.
//! The trees are placed parent first, so every lead stores its CEO as parent
//! (spec A10).

mod support;

use std::collections::HashSet;
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
use office_harnessd::server::ServerOptions;
use serde_json::{Value, json};
use support::fake_cli::FakeClis;
use support::office_client::{Client, Office, wait_for};

/// The real server binary that the fake CLIs run as the helper.
const HELPER: &str = env!("CARGO_BIN_EXE_office-harnessd");

const SUBTREE: &str = "agent.subtree";

/// The text every member prints before the reads (SC-002).
const MARKER: &str = "marker-7Q";

/// The only keys an entry may carry (contract, FR-003).
const ENTRY_KEYS: [&str; 9] = [
    "agent_id",
    "held",
    "last_output_at",
    "name",
    "needs_you",
    "parent_agent_id",
    "role",
    "status",
    "task",
];

/// The longest a read may take with 30 agents in the office (SC-005).
const READ_LIMIT: Duration = Duration::from_secs(2);

/// A fake CLI body that hands its session's environment to
/// `<records>/<name>.env`, runs `rest`, and stays alive.
fn reader_body(records: &Path, name: &str, rest: &str) -> String {
    let folder = records.display();
    format!(
        "env > {folder}/{name}.part\nmv {folder}/{name}.part {folder}/{name}.env\n{rest}\nsleep 30\n"
    )
}

/// The call credential and server file of the reader's live session.
struct Reader {
    credential: String,
    server_file: String,
}

async fn reader_of(records: &Path, name: &str) -> Reader {
    let path = records.join(format!("{name}.env"));
    let env = wait_for("the reader's environment", || {
        fs::read_to_string(&path).ok()
    })
    .await;
    let value = |variable: &str| {
        env.lines()
            .find_map(|line| line.strip_prefix(&format!("{variable}=")))
            .expect(variable)
            .to_owned()
    };
    Reader {
        credential: value("OFFICE_HARNESS_AGENT_TOKEN"),
        server_file: value("OFFICE_HARNESS_SERVER_FILE"),
    }
}

/// Runs the helper as the reader, on a blocking thread: waiting for it on
/// the runtime thread would stop the server it talks to.
async fn call(reader: &Reader, kind: &str, fields: &Value) -> Output {
    let kind = kind.to_owned();
    let credential = reader.credential.clone();
    let server_file = reader.server_file.clone();
    let fields = fields.to_string();
    tokio::task::spawn_blocking(move || {
        Command::new(HELPER)
            .args(["agent-call", &kind, &fields])
            .env("OFFICE_HARNESS_AGENT_TOKEN", credential)
            .env("OFFICE_HARNESS_SERVER_FILE", server_file)
            .output()
            .expect("helper")
    })
    .await
    .expect("helper task")
}

fn answer_of(outcome: &Output) -> Value {
    serde_json::from_str(String::from_utf8_lossy(&outcome.stdout).trim()).expect("json")
}

/// The entries of a successful read.
async fn read_subtree(reader: &Reader) -> Vec<Value> {
    let outcome = call(reader, SUBTREE, &json!({})).await;
    let answer = answer_of(&outcome);
    assert_eq!(answer["ok"], true, "{answer}");
    answer["result"].as_array().expect("entries").clone()
}

fn ids_of(entries: &[Value]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| entry["agent_id"].as_str().expect("id").to_owned())
        .collect()
}

fn entry_of<'entries>(entries: &'entries [Value], agent_id: &str) -> &'entries Value {
    entries
        .iter()
        .find(|entry| entry["agent_id"] == agent_id)
        .expect("the agent is listed")
}

/// Places an agent through the same store the server built: the wire has no
/// placement request, so the test writes the place directly, as
/// `agent_directive.rs` does.
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

async fn actions_of(client: &mut Client, agent_id: &str) -> Vec<Value> {
    let actions = client
        .ask("agent.actions", json!({"agent_id": agent_id}))
        .await;
    assert_eq!(actions["ok"], true, "{actions}");
    actions["result"].as_array().expect("entries").clone()
}

/// Creates a placed agent whose CLI dumps its environment under its name,
/// runs `rest`, and stays alive.
async fn placed_agent(
    office: &Office,
    client: &mut Client,
    name: &str,
    role: Role,
    parent: Option<&str>,
    rest: &str,
) -> String {
    office
        .clis
        .add(name, &reader_body(office.records_path(), name, rest));
    let agent_id = client.create_agent(office, name, name, None).await;
    place(office, &agent_id, role, parent);
    agent_id
}

/// The tree of SC-001: a CEO, two leads, three members under each lead.
struct Company {
    ceo: String,
    leads: Vec<String>,
    /// The members of each lead, in the order of `leads`.
    members: Vec<Vec<String>>,
    /// Every agent as `(name, id)`; the name is also its CLI's and its
    /// environment file's.
    agents: Vec<(String, String)>,
}

impl Company {
    fn descendants_of(&self, agent_id: &str) -> HashSet<String> {
        if agent_id == self.ceo {
            return self
                .members
                .iter()
                .flatten()
                .chain(&self.leads)
                .cloned()
                .collect();
        }
        self.leads
            .iter()
            .position(|lead| lead == agent_id)
            .map_or_else(HashSet::new, |team| {
                self.members[team].iter().cloned().collect()
            })
    }
}

async fn company(office: &Office, client: &mut Client) -> Company {
    let member_output = format!("echo {MARKER}");
    let ceo = placed_agent(office, client, "ceo", Role::Ceo, None, "").await;
    let mut company = Company {
        ceo: ceo.clone(),
        leads: Vec::new(),
        members: Vec::new(),
        agents: vec![("ceo".to_owned(), ceo.clone())],
    };
    for team in ["a", "b"] {
        let lead_name = format!("lead-{team}");
        let lead = placed_agent(office, client, &lead_name, Role::Lead, Some(&ceo), "").await;
        company.agents.push((lead_name, lead.clone()));
        let mut members = Vec::new();
        for index in 0..3 {
            let name = format!("member-{team}{index}");
            let member = placed_agent(
                office,
                client,
                &name,
                Role::Member,
                Some(&lead),
                &member_output,
            )
            .await;
            company.agents.push((name, member.clone()));
            members.push(member);
        }
        company.leads.push(lead);
        company.members.push(members);
    }
    for (_, agent_id) in &company.agents {
        client.start(agent_id).await;
    }
    company
}

#[tokio::test]
async fn each_of_nine_readers_gets_exactly_its_descendants_and_no_terminal_output() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let company = company(&office, &mut client).await;
    for member in company.members.iter().flatten() {
        client.wait_for_status(member, "waiting_for_input").await;
    }

    for (name, agent_id) in &company.agents {
        let reader = reader_of(office.records_path(), name).await;
        let outcome = call(&reader, SUBTREE, &json!({})).await;
        let stdout = String::from_utf8_lossy(&outcome.stdout).into_owned();
        assert!(
            !stdout.contains(MARKER),
            "{name} read terminal output: {stdout}"
        );
        let answer = answer_of(&outcome);
        assert_eq!(answer["ok"], true, "{answer}");
        let entries = answer["result"].as_array().expect("entries");
        let read: HashSet<String> = ids_of(entries).into_iter().collect();
        assert_eq!(read, company.descendants_of(agent_id), "{name}");
        assert!(!read.contains(agent_id.as_str()), "{name} read itself");

        let mut seen = HashSet::from([agent_id.clone()]);
        for entry in entries {
            let mut keys: Vec<&str> = entry
                .as_object()
                .expect("entry")
                .keys()
                .map(String::as_str)
                .collect();
            keys.sort_unstable();
            assert_eq!(keys, ENTRY_KEYS, "{entry}");
            let parent = entry["parent_agent_id"].as_str().expect("parent");
            assert!(
                seen.contains(parent),
                "{name}: an entry came before its parent"
            );
            seen.insert(entry["agent_id"].as_str().expect("id").to_owned());
        }
    }
    office.serving.stop().await;
}

#[tokio::test]
async fn a_lead_sees_each_status_change_of_its_member() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let records = office.records_path();
    let quiet_file = records.join("go-quiet");
    let ceo = placed_agent(&office, &mut client, "ceo", Role::Ceo, None, "").await;
    let lead = placed_agent(&office, &mut client, "lead", Role::Lead, Some(&ceo), "").await;
    let chatty = format!(
        "while [ ! -e {} ]; do echo tick; sleep 0.1; done",
        quiet_file.display()
    );
    let member = placed_agent(
        &office,
        &mut client,
        "member",
        Role::Member,
        Some(&lead),
        &chatty,
    )
    .await;
    client.start(&lead).await;
    client.start(&member).await;
    let reader = reader_of(records, "lead").await;

    client.wait_for_status(&member, "working").await;
    let working = read_subtree(&reader).await;
    assert_eq!(entry_of(&working, &member)["status"], "working");
    assert!(entry_of(&working, &member)["last_output_at"].is_i64());

    fs::write(&quiet_file, "").expect("quiet signal");
    client.wait_for_status(&member, "waiting_for_input").await;
    let waiting = read_subtree(&reader).await;
    assert_eq!(entry_of(&waiting, &member)["status"], "waiting_for_input");

    let stopped = client.ask("agent.stop", json!({"agent_id": member})).await;
    assert_eq!(stopped["ok"], true, "{stopped}");
    client.wait_for_status(&member, "stopped").await;
    let after_stop = read_subtree(&reader).await;
    assert_eq!(entry_of(&after_stop, &member)["status"], "stopped");
    assert_eq!(
        entry_of(&after_stop, &member)["last_output_at"],
        Value::Null
    );
    office.serving.stop().await;
}

#[tokio::test]
async fn a_read_shows_an_asleep_member_as_asleep() {
    let office = Office::open_with_options(
        FakeClis::new(),
        ServerOptions {
            idle_sleep_tick: Duration::from_millis(50),
            idle_sleep_minute: Duration::from_millis(100),
            ..ServerOptions::default()
        },
    )
    .await;
    let mut client = office.client().await;
    let records = office.records_path();
    let ceo = placed_agent(&office, &mut client, "ceo", Role::Ceo, None, "").await;
    let member = placed_agent(
        &office,
        &mut client,
        "member",
        Role::Lead,
        Some(&ceo),
        "echo ready",
    )
    .await;
    client.start(&ceo).await;
    client.start(&member).await;
    let reader = reader_of(records, "ceo").await;
    client.wait_for_status(&member, "waiting_for_input").await;
    // The reader's own Needs you item stays, so only the member sleeps.
    client.dismiss_needs_you(&member).await;

    client.set_setting("idle_sleep_minutes", "5").await;
    client.wait_for_status(&member, "asleep").await;

    let entries = read_subtree(&reader).await;
    assert_eq!(entry_of(&entries, &member)["status"], "asleep");
    office.serving.stop().await;
}

#[tokio::test]
async fn reads_pass_holds_count_toward_no_limit_and_leave_no_record() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let ceo = placed_agent(&office, &mut client, "ceo", Role::Ceo, None, "").await;
    let lead = placed_agent(&office, &mut client, "lead", Role::Lead, Some(&ceo), "").await;
    client.start(&lead).await;
    client.start(&ceo).await;
    let reader = reader_of(office.records_path(), "ceo").await;

    let held = client.ask("hold.set", json!({})).await;
    assert_eq!(held["ok"], true, "{held}");
    for _ in 0..10 {
        let entries = read_subtree(&reader).await;
        assert_eq!(ids_of(&entries), std::slice::from_ref(&lead));
        assert_eq!(entries[0]["held"], true);
    }
    let released = client.ask("hold.release", json!({})).await;
    assert_eq!(released["ok"], true, "{released}");

    let recorded_before = actions_of(&mut client, &ceo).await.len();
    for _ in 0..30 {
        read_subtree(&reader).await;
    }
    assert_eq!(actions_of(&mut client, &ceo).await.len(), recorded_before);

    let directive = call(
        &reader,
        "directive.send",
        &json!({"target_agent_id": lead, "text": "status please"}),
    )
    .await;
    assert_eq!(
        answer_of(&directive)["ok"],
        true,
        "{}",
        answer_of(&directive)
    );
    let holds = client.ask("hold.list", json!({})).await;
    assert_eq!(holds["result"], json!({"office": null, "agents": []}));
    office.serving.stop().await;
}

#[tokio::test]
async fn a_read_that_names_another_agent_is_refused_and_recorded() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let ceo = placed_agent(&office, &mut client, "ceo", Role::Ceo, None, "").await;
    let lead_a = placed_agent(&office, &mut client, "lead-a", Role::Lead, Some(&ceo), "").await;
    let lead_b = placed_agent(&office, &mut client, "lead-b", Role::Lead, Some(&ceo), "").await;
    let member_b = placed_agent(
        &office,
        &mut client,
        "member-b",
        Role::Member,
        Some(&lead_b),
        "",
    )
    .await;
    client.start(&lead_a).await;
    let reader = reader_of(office.records_path(), "lead-a").await;
    let recorded_before = actions_of(&mut client, &lead_a).await.len();

    let others = [&lead_b, &ceo, &member_b];
    let keys = ["agent_id", "target_agent_id", "root_agent_id"];
    for attempt in 0..10 {
        let other = others[attempt % others.len()];
        let fields = json!({ keys[attempt % keys.len()]: other });
        let outcome = call(&reader, SUBTREE, &fields).await;
        let stdout = String::from_utf8_lossy(&outcome.stdout).into_owned();
        let answer = answer_of(&outcome);
        assert_eq!(answer["ok"], false, "{answer}");
        assert_eq!(answer["error"]["code"], "invalid", "{answer}");
        assert!(!stdout.contains(other.as_str()), "{stdout}");
        assert!(
            !stdout.contains("lead-b") && !stdout.contains("member-b"),
            "{stdout}"
        );
    }
    let refusals = actions_of(&mut client, &lead_a).await.len() - recorded_before;
    assert_eq!(refusals, 10);
    office.serving.stop().await;
}

#[tokio::test]
async fn a_task_line_reaches_the_reader_as_one_line_of_visible_symbols() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let ceo = placed_agent(&office, &mut client, "ceo", Role::Ceo, None, "").await;
    let lead = placed_agent(&office, &mut client, "lead", Role::Lead, Some(&ceo), "").await;
    let set = client
        .ask(
            "agent.task.set",
            json!({"agent_id": lead, "text": "\u{1b}[31m\u{9b}\u{202e}red\nsecond"}),
        )
        .await;
    assert_eq!(set["ok"], true, "{set}");
    client.start(&ceo).await;
    let reader = reader_of(office.records_path(), "ceo").await;

    let outcome = call(&reader, SUBTREE, &json!({})).await;
    let stdout = String::from_utf8_lossy(&outcome.stdout).into_owned();
    for raw in ['\u{1b}', '\u{9b}', '\u{202e}'] {
        assert!(
            !stdout.contains(raw),
            "{raw:?} reached the reader: {stdout:?}"
        );
    }
    let answer = answer_of(&outcome);
    let task = &answer["result"][0]["task"];
    assert_eq!(task["line"], "␛[31m\\u009B\\u202Ered…");
    assert!(task["set_at"].is_i64());
    assert_eq!(task.as_object().expect("task").len(), 2, "{task}");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_read_shows_the_needs_you_and_hold_flags_and_changes_neither() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let ceo = placed_agent(&office, &mut client, "ceo", Role::Ceo, None, "").await;
    let lead = placed_agent(
        &office,
        &mut client,
        "lead",
        Role::Lead,
        Some(&ceo),
        "echo ready",
    )
    .await;
    client.start(&ceo).await;
    client.start(&lead).await;
    client.wait_for_status(&lead, "waiting_for_input").await;
    let reader = reader_of(office.records_path(), "ceo").await;
    let held = client.ask("hold.set", json!({"agent_id": lead})).await;
    assert_eq!(held["ok"], true, "{held}");
    let items_before = client.ask("needs_you.list", json!({})).await;

    let entries = read_subtree(&reader).await;
    let entry = entry_of(&entries, &lead);
    assert_eq!(entry["needs_you"], true, "{entry}");
    assert_eq!(entry["held"], true, "{entry}");

    let items_after = client.ask("needs_you.list", json!({})).await;
    assert_eq!(items_after["result"], items_before["result"]);
    assert_eq!(client.status_of(&lead).await, "waiting_for_input");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_read_answers_within_two_seconds_with_thirty_agents() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let ceo = placed_agent(&office, &mut client, "ceo", Role::Ceo, None, "").await;
    let mut leads = Vec::new();
    for index in 0..2 {
        let name = format!("lead-{index}");
        leads.push(placed_agent(&office, &mut client, &name, Role::Lead, Some(&ceo), "").await);
    }
    for index in 0..27 {
        let name = format!("member-{index}");
        let lead = &leads[index % leads.len()];
        placed_agent(&office, &mut client, &name, Role::Member, Some(lead), "").await;
    }
    client.start(&ceo).await;
    let reader = reader_of(office.records_path(), "ceo").await;

    let started = Instant::now();
    let entries = read_subtree(&reader).await;
    assert!(started.elapsed() < READ_LIMIT, "{:?}", started.elapsed());
    assert_eq!(entries.len(), 29);
    office.serving.stop().await;
}

#[tokio::test]
async fn a_failed_read_answers_internal_raises_the_storage_alert_and_is_recorded() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let ceo = placed_agent(&office, &mut client, "ceo", Role::Ceo, None, "").await;
    client.start(&ceo).await;
    let reader = reader_of(office.records_path(), "ceo").await;
    let recorded_before = actions_of(&mut client, &ceo).await.len();
    // A side connection hides the Needs you table, so the read fails at
    // its third source.
    rusqlite::Connection::open(office.serving.office.config.database_path())
        .expect("database")
        .execute_batch("ALTER TABLE notification_needs_you RENAME TO hidden_needs_you")
        .expect("rename");

    let answer = answer_of(&call(&reader, SUBTREE, &json!({})).await);
    assert_eq!(answer["error"]["code"], "internal", "{answer}");
    let alert = client.next_event("storage.error").await;
    assert!(
        alert["message"]
            .as_str()
            .expect("message")
            .contains("no such table"),
        "{alert}"
    );
    assert_eq!(
        actions_of(&mut client, &ceo).await.len(),
        recorded_before + 1
    );
    office.serving.stop().await;
}

#[tokio::test]
async fn the_desktop_connection_does_not_offer_the_read() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let answer = client.ask(SUBTREE, json!({})).await;
    assert_eq!(answer["ok"], false, "{answer}");
    assert_eq!(answer["error"]["code"], "not_found", "{answer}");
    office.serving.stop().await;
}
