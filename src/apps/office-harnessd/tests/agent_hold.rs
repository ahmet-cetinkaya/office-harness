// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! The hold switch and the agent action limits (0007), against a real
//! server and a real agent-call connection: the fake CLI of the lead dumps
//! the credential of its live session, and the test calls the helper with
//! it.

mod support;

use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use agent_infrastructure::persistence::SharedConnection;
use hierarchy_application::common::interfaces::HierarchyStore;
use hierarchy_domain::{AgentAction, Role};
use hierarchy_infrastructure::persistence::repositories::hierarchy_repository::HierarchyRepository;
use hierarchy_infrastructure::services::agent_places::AgentPlaces;
use kernel::AgentId;
use office_harnessd::bootstrap;
use serde_json::{Value, json};
use support::office_client::{Client, Office, error_text, wait_for};

/// The real server binary that the fake CLI runs as the helper.
const HELPER: &str = env!("CARGO_BIN_EXE_office-harnessd");

const OFFICE_HOLD_REASON: &str = "the office is on hold";
const DIRECTIVE_LIMIT_NAME: &str = "directives sent by one agent";
const SMALL_DIRECTIVE_LIMIT: u32 = 2;
const DEFAULT_DIRECTIVE_LIMIT: usize = 10;
const SC001_DIRECTIVES: usize = 100;
const WIDE_TARGET_LIMIT: &str = "1000";
const REFUSALS_BEFORE_AUTO_HOLD: usize = 3;
const STOPPED_TARGET_DIRECTIVES: usize = 12;

/// A team lead that calls the office from its live session, and the member
/// under it.
struct Setup {
    office: Office,
    client: Client,
    lead: String,
    member: String,
    token: String,
    server_file: String,
}

/// A fake CLI that hands its environment to the records folder and stays
/// alive, so the test holds the credential of its live session.
fn env_cli(records: &Path) -> String {
    let folder = records.display();
    format!("env > {folder}/env.part\nmv {folder}/env.part {folder}/env\nsleep 60\n")
}

fn store_of(office: &Office) -> Arc<dyn HierarchyStore> {
    let connection =
        rusqlite::Connection::open(office.serving.office.config.database_path()).expect("database");
    Arc::new(HierarchyRepository::new(
        SharedConnection::new(connection).handle(),
        AgentPlaces::new(),
    ))
}

/// Gives the agent a team, which makes it a lead (as `agent_call.rs` does).
fn place_lead_with_team(office: &Office, lead: &str) {
    hierarchy_application::CreateTeamHandler::new(store_of(office))
        .handle(
            &hierarchy_application::commands::create_team::CreateTeamRequest {
                name: "The Team".to_owned(),
                project_id: None,
                lead_agent_id: lead.parse().expect("agent id"),
            },
        )
        .expect("team");
}

fn place_member(office: &Office, member: &str, lead: &str) {
    hierarchy_application::PlaceAgentHandler::new(store_of(office))
        .handle(
            hierarchy_application::commands::place_agent::PlaceAgentRequest {
                agent_id: member.parse().expect("agent id"),
                role: Role::Member,
                parent_agent_id: Some(lead.parse().expect("agent id")),
                team_id: None,
            },
        )
        .expect("placement");
}

async fn session_environment(records: &Path) -> (String, String) {
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

async fn setup() -> Setup {
    let office = Office::open().await;
    office.clis.add("member-cli", "echo ready; sleep 60");
    office.clis.add("lead-cli", &env_cli(office.records_path()));
    let mut client = office.client().await;
    let lead = client.create_agent(&office, "Lead", "lead-cli", None).await;
    place_lead_with_team(&office, &lead);
    let member = client
        .create_agent(&office, "Member", "member-cli", None)
        .await;
    place_member(&office, &member, &lead);
    client.start(&member).await;
    client.wait_for_status(&member, "working").await;
    client.start(&lead).await;
    let (token, server_file) = session_environment(office.records_path()).await;
    Setup {
        office,
        client,
        lead,
        member,
        token,
        server_file,
    }
}

impl Setup {
    /// One agent-call request from the lead's live session, on a blocking
    /// thread (the server runs on this test's runtime).
    async fn call(&self, kind: &str, request: &Value) -> Value {
        let (kind, request) = (kind.to_owned(), request.to_string());
        let (token, server_file) = (self.token.clone(), self.server_file.clone());
        let output = tokio::task::spawn_blocking(move || {
            Command::new(HELPER)
                .args(["agent-call", &kind, &request])
                .env("OFFICE_HARNESS_AGENT_TOKEN", token)
                .env("OFFICE_HARNESS_SERVER_FILE", server_file)
                .output()
                .expect("helper")
        })
        .await
        .expect("helper task");
        serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim()).expect("json")
    }

    async fn directive(&self, text: &str) -> Value {
        self.call(
            "directive.send",
            &json!({"target_agent_id": self.member, "text": text}),
        )
        .await
    }

    async fn ask(&mut self, kind: &str, fields: Value) -> Value {
        self.client.ask(kind, fields).await
    }

    async fn set_setting(&mut self, key: &str, value: &str) {
        let answer = self
            .ask("settings.set", json!({"key": key, "value": value}))
            .await;
        assert_eq!(answer["ok"], true, "{answer}");
    }

    async fn holds(&mut self) -> Value {
        let listed = self.ask("hold.list", json!({})).await;
        assert_eq!(listed["ok"], true, "{listed}");
        listed["result"].clone()
    }

    async fn actions_of(&mut self, agent_id: &str) -> Vec<Value> {
        let actions = self
            .ask("agent.actions", json!({"agent_id": agent_id}))
            .await;
        assert_eq!(actions["ok"], true, "{actions}");
        actions["result"].as_array().expect("entries").clone()
    }
}

fn refusal_message(answer: &Value) -> &str {
    let (code, message) = error_text(answer);
    assert_eq!(code, "refused", "{answer}");
    message
}

/// The five requests of an agent that the gate covers, as the lead sends
/// them to its member.
fn five_agent_requests(setup: &Setup) -> Vec<(&'static str, Value)> {
    let member = &setup.member;
    vec![
        (
            "directive.send",
            json!({"target_agent_id": member, "text": "hello"}),
        ),
        (
            "agent.create",
            json!({
                "name": "Kid",
                "command": "member-cli",
                "working_folder": setup.office.work_path(),
                "role": {"kind": "member", "parent_agent_id": setup.lead},
            }),
        ),
        ("agent.start", json!({"agent_id": member})),
        ("agent.stop", json!({"agent_id": member})),
        ("agent.new_task", json!({"agent_id": member})),
    ]
}

#[tokio::test]
async fn an_office_hold_refuses_every_agent_kind_while_the_user_acts_and_types() {
    let mut setup = setup().await;
    let held = setup.ask("hold.set", json!({})).await;
    assert_eq!(held["ok"], true, "{held}");
    assert_eq!(held["result"]["hold"]["set_by"], "user", "{held}");

    for (kind, request) in five_agent_requests(&setup) {
        let answer = setup.call(kind, &request).await;
        assert_eq!(refusal_message(&answer), OFFICE_HOLD_REASON, "{kind}");
    }
    assert_eq!(
        setup.holds().await["agents"],
        json!([]),
        "hold refusals never hold an agent"
    );

    // The terminals keep running: the user types into the member.
    let member = setup.member.clone();
    let attached = setup
        .ask(
            "terminal.attach",
            json!({"agent_id": member, "cols": 80, "rows": 24}),
        )
        .await;
    assert_eq!(attached["ok"], true, "{attached}");
    let typed = setup
        .ask(
            "terminal.input",
            json!({"agent_id": member, "data": "aGkK"}),
        )
        .await;
    assert_eq!(typed["ok"], true, "{typed}");

    // The user's own actions are never held (PB-1, FR-005).
    let office = &setup.office;
    let extra = setup
        .client
        .create_agent(office, "Extra", "member-cli", None)
        .await;
    setup.client.start(&extra).await;
    setup.client.wait_for_status(&extra, "working").await;
    let new_task = setup
        .ask("agent.new_task", json!({"agent_id": extra}))
        .await;
    assert_eq!(new_task["ok"], true, "{new_task}");
    let stopped = setup.ask("agent.stop", json!({"agent_id": extra})).await;
    assert_eq!(stopped["ok"], true, "{stopped}");
    setup.client.wait_for_status(&extra, "stopped").await;
    let deleted = setup.ask("agent.delete", json!({"agent_id": extra})).await;
    assert_eq!(deleted["ok"], true, "{deleted}");
}

#[tokio::test]
async fn a_directive_to_a_non_descendant_while_the_office_is_held_gets_the_tree_reason() {
    let mut setup = setup().await;
    let office = &setup.office;
    let stranger = setup
        .client
        .create_agent(office, "Stranger", "member-cli", None)
        .await;
    setup.ask("hold.set", json!({})).await;

    let answer = setup
        .call(
            "directive.send",
            &json!({"target_agent_id": stranger, "text": "hi"}),
        )
        .await;

    let message = refusal_message(&answer);
    assert!(message.contains("not a descendant"), "{message}");
    assert!(!message.contains("on hold"), "{message}");
}

#[tokio::test]
async fn directives_to_a_stopped_member_are_refused_by_the_tree_and_never_hold_the_lead() {
    let mut setup = setup().await;
    let member = setup.member.clone();
    let stopped = setup.ask("agent.stop", json!({"agent_id": member})).await;
    assert_eq!(stopped["ok"], true, "{stopped}");
    setup.client.wait_for_status(&member, "stopped").await;

    for _ in 0..STOPPED_TARGET_DIRECTIVES {
        let answer = setup.directive("hello").await;
        let message = refusal_message(&answer);
        assert!(!message.contains("limit"), "{message}");
        assert!(!message.contains("hold"), "{message}");
    }

    assert_eq!(setup.holds().await["agents"], json!([]));
}

#[tokio::test]
async fn a_looping_agent_is_limited_then_held_and_the_owner_is_told_exactly_once() {
    let mut setup = setup().await;
    // A second client only listens: `ask` drops the events it skips.
    let mut watcher = setup.office.client().await;
    setup
        .set_setting(
            "limit_directives_per_agent",
            &SMALL_DIRECTIVE_LIMIT.to_string(),
        )
        .await;
    for _ in 0..SMALL_DIRECTIVE_LIMIT {
        let answer = setup.directive("go").await;
        assert_eq!(answer["ok"], true, "{answer}");
    }

    let limit_reason = format!(
        "the limit \"{DIRECTIVE_LIMIT_NAME}\" allows {SMALL_DIRECTIVE_LIMIT} per 60 seconds"
    );
    for _ in 0..REFUSALS_BEFORE_AUTO_HOLD {
        let answer = setup.directive("go").await;
        assert_eq!(refusal_message(&answer), limit_reason);
    }
    let hold_reason = format!("the agent {} is on hold", setup.lead);
    for _ in 0..2 {
        let answer = setup.directive("go").await;
        assert_eq!(refusal_message(&answer), hold_reason);
    }

    let holds = setup.holds().await;
    let agents = holds["agents"].as_array().expect("agents");
    assert_eq!(agents.len(), 1, "{holds}");
    assert_eq!(agents[0]["agent_id"], json!(setup.lead), "{holds}");
    assert_eq!(agents[0]["set_by"], "office", "{holds}");
    assert!(
        agents[0]["reason"]
            .as_str()
            .expect("reason")
            .contains(DIRECTIVE_LIMIT_NAME),
        "{holds}"
    );
    let changed = watcher.next_event("hold.changed").await;
    assert_eq!(
        changed["agents"].as_array().expect("agents").len(),
        1,
        "{changed}"
    );

    let listed = setup.ask("notification.list", json!({})).await;
    let auto_holds: Vec<&Value> = listed["result"]
        .as_array()
        .expect("rows")
        .iter()
        .filter(|row| row["event"] == "auto_hold")
        .collect();
    assert_eq!(auto_holds.len(), 1, "{listed}");
    assert_eq!(auto_holds[0]["detail"], DIRECTIVE_LIMIT_NAME, "{listed}");
    assert_eq!(auto_holds[0]["agent_id"], json!(setup.lead), "{listed}");

    let lead = setup.lead.clone();
    let lead_entries = setup.actions_of(&lead).await;
    let hold_entry = lead_entries
        .iter()
        .find(|entry| entry["action"] == "hold")
        .expect("the automatic hold is recorded");
    assert_eq!(hold_entry["actor"], "office", "{hold_entry}");
}

/// True for the entry of a directive that left the queue untyped (0012).
fn is_withdrawal(entry: &Value) -> bool {
    entry["result"]
        .as_str()
        .is_some_and(|result| result.starts_with("withdrawn before typing"))
}

#[tokio::test]
async fn a_hundred_directives_deliver_at_most_the_limit_and_every_refusal_is_recorded() {
    let mut setup = setup().await;
    // The per-target limit of 3 would refuse first; widen it so the sender
    // limit of 10 is the one under test.
    setup
        .set_setting("limit_directives_per_target", WIDE_TARGET_LIMIT)
        .await;

    let mut answers = Vec::new();
    for round in 0..SC001_DIRECTIVES {
        answers.push(setup.directive(&format!("directive {round}")).await);
    }

    let delivered = answers.iter().filter(|answer| answer["ok"] == true).count();
    assert_eq!(delivered, DEFAULT_DIRECTIVE_LIMIT);
    let messages: Vec<&str> = answers
        .iter()
        .filter(|answer| answer["ok"] != true)
        .map(refusal_message)
        .collect();
    assert_eq!(messages.len(), SC001_DIRECTIVES - DEFAULT_DIRECTIVE_LIMIT);
    let limit_reason = format!("the limit \"{DIRECTIVE_LIMIT_NAME}\" allows 10 per 60 seconds");
    let (before_hold, after_hold) = messages.split_at(REFUSALS_BEFORE_AUTO_HOLD);
    assert!(before_hold.iter().all(|message| *message == limit_reason));
    let hold_reason = format!("the agent {} is on hold", setup.lead);
    assert!(after_hold.iter().all(|message| *message == hold_reason));

    let member = setup.member.clone();
    let entries = setup.actions_of(&member).await;
    // The automatic hold also withdraws the accepted directives that still
    // wait for the member, each with its own entry (0012/FR-005); this test
    // counts the attempts at sending.
    let directives: Vec<&Value> = entries
        .iter()
        .filter(|entry| entry["action"] == "directive" && !is_withdrawal(entry))
        .collect();
    assert_eq!(directives.len(), SC001_DIRECTIVES);
    let actor = json!(format!("agent:{}", setup.lead));
    assert!(directives.iter().all(|entry| entry["actor"] == actor));
    let refused = directives
        .iter()
        .filter(|entry| entry["result"] != "ok")
        .count();
    assert_eq!(refused, SC001_DIRECTIVES - DEFAULT_DIRECTIVE_LIMIT);

    // FR-010: the sender's own record lists each limit and hold refusal too,
    // with the same reasons as the answers. Withdrawals of waiting directives
    // are recorded there as well (0012/FR-005) and are left out here, as above.
    let lead = setup.lead.clone();
    let sender_refusals: Vec<String> = setup
        .actions_of(&lead)
        .await
        .iter()
        .filter(|entry| {
            entry["action"] == "directive" && entry["result"] != "ok" && !is_withdrawal(entry)
        })
        .map(|entry| entry["result"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(
        sender_refusals.len(),
        SC001_DIRECTIVES - DEFAULT_DIRECTIVE_LIMIT
    );
    assert!(
        sender_refusals
            .iter()
            .any(|result| result.contains(DIRECTIVE_LIMIT_NAME)),
        "{sender_refusals:?}"
    );
    assert!(
        sender_refusals
            .iter()
            .any(|result| result.contains("is on hold")),
        "{sender_refusals:?}"
    );
}

#[tokio::test]
async fn an_agent_hold_refuses_an_action_on_the_held_agent_and_a_double_hold_conflicts() {
    let mut setup = setup().await;
    let member = setup.member.clone();
    let held = setup.ask("hold.set", json!({"agent_id": member})).await;
    assert_eq!(held["ok"], true, "{held}");

    let answer = setup.directive("hello").await;
    assert_eq!(
        refusal_message(&answer),
        format!("the agent {member} is on hold")
    );

    let twice = setup.ask("hold.set", json!({"agent_id": member})).await;
    assert_eq!(error_text(&twice).0, "conflict", "{twice}");
    assert_eq!(error_text(&twice).1, "the agent is already on hold");
    let unknown = setup
        .ask(
            "hold.set",
            json!({"agent_id": AgentId::generate().to_string()}),
        )
        .await;
    assert_eq!(error_text(&unknown).0, "not_found", "{unknown}");

    let released = setup.ask("hold.release", json!({"agent_id": member})).await;
    assert_eq!(released["ok"], true, "{released}");
    let again = setup.ask("hold.release", json!({"agent_id": member})).await;
    assert_eq!(error_text(&again).0, "conflict", "{again}");
    let delivered = setup.directive("hello").await;
    assert_eq!(delivered["ok"], true, "{delivered}");
}

#[tokio::test]
async fn deleting_a_held_agent_leaves_no_hold() {
    let mut setup = setup().await;
    let member = setup.member.clone();
    setup.ask("hold.set", json!({"agent_id": member})).await;
    let stopped = setup.ask("agent.stop", json!({"agent_id": member})).await;
    assert_eq!(stopped["ok"], true, "{stopped}");
    setup.client.wait_for_status(&member, "stopped").await;
    // Connects after the hold, so the first `hold.changed` it sees is the
    // delete's.
    let mut watcher = setup.office.client().await;

    let deleted = setup.ask("agent.delete", json!({"agent_id": member})).await;
    assert_eq!(deleted["ok"], true, "{deleted}");

    assert_eq!(setup.holds().await["agents"], json!([]));
    let changed = watcher.next_event("hold.changed").await;
    assert_eq!(changed["agents"], json!([]), "{changed}");
}

#[tokio::test]
async fn a_misspelt_agent_id_is_refused_and_never_releases_the_office_hold() {
    let mut setup = setup().await;
    let member = setup.member.clone();
    setup.ask("hold.set", json!({})).await;

    let typo = setup.ask("hold.release", json!({"agentid": member})).await;
    assert_eq!(error_text(&typo).0, "invalid", "{typo}");
    assert_eq!(
        error_text(&typo).1,
        "agentid is not a field of hold.release"
    );
    let listed = setup.ask("hold.list", json!({"extra": true})).await;
    assert_eq!(error_text(&listed).0, "invalid", "{listed}");

    assert_eq!(setup.holds().await["office"]["set_by"], "user");
}

#[tokio::test]
async fn an_agent_connection_asking_for_a_hold_is_refused_as_not_allowed() {
    let setup = setup().await;

    let answer = setup.call("hold.set", &json!({})).await;

    assert!(
        refusal_message(&answer).contains("an agent may only create, start, stop"),
        "{answer}"
    );
}

#[tokio::test]
async fn a_hold_set_before_a_restart_still_refuses_every_agent_action_after_it() {
    let mut setup = setup().await;
    let held = setup.ask("hold.set", json!({})).await;
    assert_eq!(held["ok"], true, "{held}");
    let (lead, member) = (setup.lead.clone(), setup.member.clone());
    let clis = setup.office.clis.login_setup();
    let stopped = setup.office.serving.stop().await;

    let services = bootstrap::start_with(&stopped.config, &stopped.log, clis).expect("restart");

    let holds = Arc::clone(&services.hierarchy.holds);
    assert!(holds.list().office.is_some());
    let rows = services.hierarchy.tree.store_snapshot().expect("tree");
    let (lead, member): (AgentId, AgentId) = (
        lead.parse().expect("agent id"),
        member.parse().expect("agent id"),
    );
    for action in [
        AgentAction::Directive { target: member },
        AgentAction::Create,
        AgentAction::Lifecycle,
    ] {
        let refusal = holds
            .guard(lead, action, Some(member), &rows)
            .expect_err("held");
        assert_eq!(refusal.reason, OFFICE_HOLD_REASON);
    }
}
