// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! The task text of an agent (0011): a directive that the member received
//! sets it, the user sets and clears it, a new task clears it, and every
//! client learns each change from `agent.task`, without a `last_line` or a
//! `status`. The member's fake CLI records the bytes the terminal typed.

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;
use std::time::{Duration, Instant};

use agent_infrastructure::persistence::{AgentRepository, SharedConnection};
use hierarchy_application::commands::place_agent::{PlaceAgentHandler, PlaceAgentRequest};
use hierarchy_application::common::interfaces::HierarchyStore;
use hierarchy_domain::Role;
use hierarchy_infrastructure::persistence::repositories::hierarchy_repository::HierarchyRepository;
use hierarchy_infrastructure::services::agent_places::AgentPlaces;
use serde_json::{Value, json};
use support::office_client::{Client, Office, error_text, wait_for};

/// The real server binary that the fake CLIs run as the helper.
const HELPER: &str = env!("CARGO_BIN_EXE_office-harnessd");

/// The wait the spec gives a directive to reach the list (SC-001).
const TASK_EVENT_WAIT: Duration = Duration::from_secs(2);
/// How long a quiet wait lasts when a test proves that nothing was sent.
const QUIET_WAIT: Duration = Duration::from_millis(500);
/// How long the test waits for the member's first output line.
const LAST_LINE_WAIT: Duration = Duration::from_secs(10);
const LINE_POLL: Duration = Duration::from_millis(100);
/// How long the member prints output before its task text is read again.
const OUTPUT_PERIOD: Duration = Duration::from_secs(2);

const DIRECTIVE: &str = "Fix the login test. Then report.";
const TASK_LIMIT_MESSAGE: &str = "a task text has at most 8,192 characters";
const TASK_LIMIT_CHARS: usize = 8192;

/// A member that records what its terminal typed, byte by byte, and keeps
/// running.
fn typing_member(typed: &Path) -> String {
    format!("echo ready\nstty raw -echo\ncat >> {}\n", typed.display())
}

/// A member that prints output until the test ends.
const PRINTING_MEMBER: &str = "while true; do echo tick; sleep 0.1; done";

/// A lead that hands its session environment to the records folder, so the
/// test holds its live credential, and stays alive.
fn env_cli(records: &Path) -> String {
    let folder = records.display();
    format!("env > {folder}/env.part\nmv {folder}/env.part {folder}/env\nsleep 60\n")
}

/// An office with a CEO whose session credential the test holds, and a
/// started member that the CEO may direct. A second client, `watcher`, sent
/// no request, so what it receives came through the server's publish to
/// every client.
struct Setup {
    office: Office,
    client: Client,
    watcher: Client,
    member: String,
    token: String,
    server_file: String,
    typed: PathBuf,
}

impl Setup {
    async fn with_member(member_body: impl Fn(&Path) -> String) -> Self {
        let office = Office::open().await;
        let typed = office.records_path().join("typed");
        office.clis.add("member-cli", &member_body(&typed));
        office.clis.add("lead-cli", &env_cli(office.records_path()));
        let mut client = office.client().await;
        let lead = client.create_agent(&office, "Lead", "lead-cli", None).await;
        let member = client
            .create_agent(&office, "Member", "member-cli", None)
            .await;
        place(&office, &lead, Role::Ceo, None);
        place(&office, &member, Role::Lead, Some(&lead));
        client.start(&member).await;
        client.wait_for_status(&member, "working").await;
        client.start(&lead).await;
        let (token, server_file) = live_credential(office.records_path()).await;
        let watcher = office.client().await;
        Self {
            office,
            client,
            watcher,
            member,
            token,
            server_file,
            typed,
        }
    }

    async fn new() -> Self {
        Self::with_member(typing_member).await
    }

    async fn directive(&self, text: &str) -> Output {
        self.directive_to(&self.member, text).await
    }

    async fn directive_to(&self, target: &str, text: &str) -> Output {
        call_helper(
            "directive.send",
            &self.token,
            &self.server_file,
            &json!({"target_agent_id": target, "text": text}),
        )
        .await
    }

    async fn set_task(&mut self, agent_id: &str, text: &str) -> Value {
        self.client
            .ask(
                "agent.task.set",
                json!({"agent_id": agent_id, "text": text}),
            )
            .await
    }

    /// The task of `agent_id` as `agent.list` shows it.
    async fn task_of(&mut self, agent_id: &str) -> Value {
        let listed = self.client.ask("agent.list", json!({})).await;
        listed["result"]
            .as_array()
            .expect("agents")
            .iter()
            .find(|agent| agent["id"] == agent_id)
            .expect("the agent is listed")["task"]
            .clone()
    }

    async fn actions_of(&mut self, agent_id: &str) -> Vec<Value> {
        let actions = self
            .client
            .ask("agent.actions", json!({"agent_id": agent_id}))
            .await;
        assert_eq!(actions["ok"], true, "{actions}");
        actions["result"].as_array().expect("entries").clone()
    }

    async fn count_actions(&mut self, agent_id: &str, action: &str) -> usize {
        self.actions_of(agent_id)
            .await
            .iter()
            .filter(|entry| entry["action"] == action)
            .count()
    }

    /// The bytes the member's terminal has typed so far.
    fn typed_bytes(&self) -> Vec<u8> {
        fs::read(&self.typed).unwrap_or_default()
    }

    async fn stop(self) {
        self.office.serving.stop().await;
    }
}

/// Places an agent through the same store the server built: the wire has no
/// placement request yet.
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
async fn call_helper(kind: &str, credential: &str, server_file: &str, request: &Value) -> Output {
    let kind = kind.to_owned();
    let credential = credential.to_owned();
    let server_file = server_file.to_owned();
    let request = request.to_string();
    tokio::task::spawn_blocking(move || {
        Command::new(HELPER)
            .args(["agent-call", &kind, &request])
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

/// The next `agent.task` event of `agent_id`, within the spec's 2 seconds.
async fn next_task_event(watcher: &mut Client, agent_id: &str) -> Value {
    tokio::time::timeout(TASK_EVENT_WAIT, async {
        loop {
            let event = watcher.next_event("agent.task").await;
            if event["agent_id"] == agent_id {
                return event;
            }
        }
    })
    .await
    .expect("agent.task within 2 seconds")
}

/// True when no further `agent.task` arrives in a quiet moment.
async fn no_task_event(watcher: &mut Client) -> bool {
    tokio::time::timeout(QUIET_WAIT, watcher.next_event("agent.task"))
        .await
        .is_err()
}

fn assert_task_only(event: &Value) {
    assert_eq!(event.get("last_line"), None, "{event}");
    assert_eq!(event.get("status"), None, "{event}");
}

/// The typed bytes split into the directives they carry, in write order.
fn typed_texts(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .split('\r')
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
        .collect()
}

#[tokio::test]
async fn a_directive_sets_the_task_and_every_client_gets_agent_task_without_line_or_status() {
    let mut setup = Setup::new().await;

    let outcome = setup.directive(DIRECTIVE).await;

    assert!(outcome.status.success(), "{outcome:?}");
    let event = next_task_event(&mut setup.watcher, &setup.member).await;
    assert_eq!(event["task"]["text"], DIRECTIVE, "{event}");
    assert_eq!(event["task"]["source"], "directive", "{event}");
    assert!(event["task"]["set_at"].as_i64().expect("set_at") > 0);
    assert_task_only(&event);
    let listed = setup.task_of(&setup.member.clone()).await;
    assert_eq!(listed, event["task"]);
    // PB-1: the terminal gets the directive bytes and the Enter, as before.
    let typed = wait_for("the typed bytes", || {
        Some(setup.typed_bytes()).filter(|bytes| !bytes.is_empty())
    })
    .await;
    assert_eq!(String::from_utf8_lossy(&typed), format!("{DIRECTIVE}\r"));
    // A4: the directive is recorded once and no task_text entry is added.
    let member = setup.member.clone();
    assert_eq!(setup.count_actions(&member, "directive").await, 1);
    assert_eq!(setup.count_actions(&member, "task_text").await, 0);
    setup.stop().await;
}

#[tokio::test]
async fn a_refused_directive_changes_no_task_text() {
    let mut setup = Setup::new().await;
    let stranger = setup
        .client
        .create_agent(&setup.office, "Stranger", "member-cli", None)
        .await;
    let member = setup.member.clone();
    for agent in [&member, &stranger] {
        assert_eq!(setup.set_task(agent, "Before").await["ok"], true);
    }
    let before_member = setup.task_of(&member).await;
    let before_stranger = setup.task_of(&stranger).await;

    let refusals = [
        setup.directive_to(&stranger, "you there?").await,
        setup.directive(" ").await,
        setup.directive("\u{1b}[D").await,
    ];
    let held = setup.client.ask("hold.set", json!({})).await;
    assert_eq!(held["ok"], true, "{held}");
    let held_refusal = setup.directive(DIRECTIVE).await;

    for outcome in refusals.iter().chain([&held_refusal]) {
        assert!(!outcome.status.success(), "{outcome:?}");
        assert_eq!(answer_of(outcome)["ok"], false);
    }
    assert_eq!(setup.task_of(&member).await, before_member);
    assert_eq!(setup.task_of(&stranger).await, before_stranger);
    assert_eq!(setup.typed_bytes().len(), 0);
    setup.stop().await;
}

#[tokio::test]
async fn directives_at_once_leave_the_text_that_was_written_last() {
    let mut setup = Setup::new().await;

    // The per-target limit of 0007 is three directives a minute.
    let (first, second, third) = tokio::join!(
        setup.directive("task one"),
        setup.directive("task two"),
        setup.directive("task three"),
    );

    for outcome in [&first, &second, &third] {
        assert!(outcome.status.success(), "{outcome:?}");
    }
    let written = wait_for("the three directives", || {
        Some(typed_texts(&setup.typed_bytes())).filter(|texts| texts.len() == 3)
    })
    .await;
    let member = setup.member.clone();
    let task = setup.task_of(&member).await;
    assert_eq!(task["text"], written.last().expect("last").as_str());
    setup.stop().await;
}

#[tokio::test]
async fn a_user_edit_racing_a_directive_leaves_both_in_the_record() {
    let mut setup = Setup::new().await;
    let member = setup.member.clone();
    let edit = json!({"agent_id": member, "text": "Edited by the user"});
    let directive_request = json!({"target_agent_id": member, "text": DIRECTIVE});

    let (directive, edited) = tokio::join!(
        call_helper(
            "directive.send",
            &setup.token,
            &setup.server_file,
            &directive_request,
        ),
        setup.client.ask("agent.task.set", edit),
    );

    assert!(directive.status.success(), "{directive:?}");
    assert_eq!(edited["ok"], true, "{edited}");
    assert_eq!(setup.count_actions(&member, "directive").await, 1);
    assert_eq!(setup.count_actions(&member, "task_text").await, 1);
    setup.stop().await;
}

#[tokio::test]
async fn a_member_that_prints_output_keeps_its_task_text() {
    let mut setup = Setup::with_member(|_| PRINTING_MEMBER.to_owned()).await;
    let member = setup.member.clone();
    setup.directive(DIRECTIVE).await;
    let event = next_task_event(&mut setup.watcher, &member).await;

    wait_for_last_line(&mut setup.client, &member).await;
    tokio::time::sleep(OUTPUT_PERIOD).await;

    assert_eq!(setup.task_of(&member).await, event["task"]);
    assert!(no_task_event(&mut setup.watcher).await);
    setup.stop().await;
}

#[tokio::test]
async fn the_user_sets_edits_and_clears_the_task_text_and_every_client_gets_it() {
    let mut setup = Setup::new().await;
    let member = setup.member.clone();

    let set = setup.set_task(&member, "Review PR 12").await;

    assert_eq!(set["ok"], true, "{set}");
    assert_eq!(set["result"]["task"]["text"], "Review PR 12");
    assert_eq!(set["result"]["task"]["source"], "user");
    let event = next_task_event(&mut setup.watcher, &member).await;
    assert_eq!(event["task"], set["result"]["task"]);
    assert_task_only(&event);

    let edited = setup.set_task(&member, "Review PR 13").await;
    assert_eq!(edited["result"]["task"]["text"], "Review PR 13");
    let edit_event = next_task_event(&mut setup.watcher, &member).await;
    assert_eq!(edit_event["task"]["text"], "Review PR 13");

    let cleared = setup.set_task(&member, "   ").await;
    assert_eq!(cleared["ok"], true, "{cleared}");
    let clear_event = next_task_event(&mut setup.watcher, &member).await;
    assert_eq!(clear_event["task"], Value::Null);
    assert_eq!(setup.task_of(&member).await, Value::Null);

    // Each change is recorded with the user as actor.
    let entries = setup.actions_of(&member).await;
    let task_entries: Vec<&Value> = entries
        .iter()
        .filter(|entry| entry["action"] == "task_text")
        .collect();
    assert_eq!(task_entries.len(), 3, "{entries:?}");
    assert!(task_entries.iter().all(|entry| entry["actor"] == "user"));
    // A2: setting the text types nothing into the terminal.
    assert_eq!(setup.typed_bytes().len(), 0);
    setup.stop().await;
}

#[tokio::test]
async fn a_task_text_of_the_limit_is_accepted_and_one_past_it_is_refused() {
    let mut setup = Setup::new().await;
    let member = setup.member.clone();

    let at_limit = setup.set_task(&member, &"x".repeat(TASK_LIMIT_CHARS)).await;
    let over = setup
        .set_task(&member, &"x".repeat(TASK_LIMIT_CHARS + 1))
        .await;
    let blank_and_long = setup
        .set_task(&member, &" ".repeat(TASK_LIMIT_CHARS + 1))
        .await;

    assert_eq!(at_limit["ok"], true, "{at_limit}");
    assert_eq!(error_text(&over), ("invalid", TASK_LIMIT_MESSAGE));
    assert_eq!(blank_and_long["ok"], true, "{blank_and_long}");
    assert_eq!(setup.task_of(&member).await, Value::Null);
    setup.stop().await;
}

#[tokio::test]
async fn a_control_character_is_stored_as_it_was_entered() {
    let mut setup = Setup::new().await;
    let member = setup.member.clone();
    let text = "red \u{1b}[31m\nsecond line";

    let set = setup.set_task(&member, text).await;

    assert_eq!(set["ok"], true, "{set}");
    assert_eq!(setup.task_of(&member).await["text"], text);
    setup.stop().await;
}

#[tokio::test]
async fn an_agent_connection_may_not_set_a_task_text() {
    let mut setup = Setup::new().await;
    let member = setup.member.clone();

    let outcome = call_helper(
        "agent.task.set",
        &setup.token,
        &setup.server_file,
        &json!({"agent_id": member, "text": "Take over"}),
    )
    .await;

    let answer = answer_of(&outcome);
    assert_eq!(answer["error"]["code"], "refused", "{answer}");
    assert_eq!(setup.task_of(&member).await, Value::Null);
    assert_eq!(setup.count_actions(&member, "task_text").await, 0);
    setup.stop().await;
}

#[tokio::test]
async fn a_new_task_clears_the_text_and_start_and_continue_keep_it() {
    let office = Office::open().await;
    office.clis.add("claude", "echo ready");
    let mut client = office.client().await;
    let mut watcher = office.client().await;
    let agent = client.create_agent(&office, "Ana", "claude", None).await;
    let set = client
        .ask(
            "agent.task.set",
            json!({"agent_id": agent, "text": "Keep me"}),
        )
        .await;
    assert_eq!(set["ok"], true, "{set}");
    next_task_event(&mut watcher, &agent).await;

    let first = client.start(&agent).await;
    client.wait_for_status(&agent, "exited").await;
    assert_eq!(task_text_of(&mut client, &agent).await, "Keep me");
    let continued = client
        .ask("session.continue", json!({"session_id": first["id"]}))
        .await;
    assert_eq!(continued["ok"], true, "{continued}");
    client.wait_for_status(&agent, "exited").await;
    assert_eq!(task_text_of(&mut client, &agent).await, "Keep me");
    assert!(no_task_event(&mut watcher).await);

    let restarted = client
        .ask("agent.new_task", json!({"agent_id": agent}))
        .await;

    assert_eq!(restarted["ok"], true, "{restarted}");
    let event = next_task_event(&mut watcher, &agent).await;
    assert_eq!(event["task"], Value::Null, "{event}");
    assert_task_only(&event);
    assert_eq!(task_text_of(&mut client, &agent).await, Value::Null);
    office.serving.stop().await;
}

async fn wait_for_last_line(client: &mut Client, agent_id: &str) {
    let deadline = Instant::now() + LAST_LINE_WAIT;
    loop {
        let listed = client.ask("agent.list", json!({})).await;
        let line = listed["result"]
            .as_array()
            .expect("agents")
            .iter()
            .find(|agent| agent["id"] == agent_id)
            .expect("the agent is listed")["last_line"]
            .clone();
        if line.is_string() {
            return;
        }
        assert!(Instant::now() < deadline, "the member showed no output");
        tokio::time::sleep(LINE_POLL).await;
    }
}

async fn task_text_of(client: &mut Client, agent_id: &str) -> Value {
    let listed = client.ask("agent.list", json!({})).await;
    listed["result"]
        .as_array()
        .expect("agents")
        .iter()
        .find(|agent| agent["id"] == agent_id)
        .expect("the agent is listed")["task"]["text"]
        .clone()
}

#[tokio::test]
async fn the_task_text_and_its_time_survive_a_restart_and_go_with_a_deleted_agent() {
    let mut setup = Setup::new().await;
    let member = setup.member.clone();
    setup.directive(DIRECTIVE).await;
    let event = next_task_event(&mut setup.watcher, &member).await;
    let doomed = setup
        .client
        .create_agent(&setup.office, "Doomed", "member-cli", None)
        .await;
    setup.set_task(&doomed, "Gone soon").await;
    let deleted = setup
        .client
        .ask("agent.delete", json!({"agent_id": doomed}))
        .await;
    assert_eq!(deleted["ok"], true, "{deleted}");

    let stopped = setup.office.serving.stop().await;

    let connection = rusqlite::Connection::open(stopped.config.database_path()).expect("database");
    let agents = AgentRepository::new(SharedConnection::new(connection));
    let stored = agents
        .find(member.parse().expect("id"))
        .expect("find")
        .expect("agent")
        .task
        .expect("task kept");
    assert_eq!(stored.text(), DIRECTIVE);
    assert_eq!(
        stored.set_at().as_unix_millis(),
        event["task"]["set_at"].as_i64().expect("set_at")
    );
    assert_eq!(
        agents.find(doomed.parse().expect("id")).expect("find"),
        None
    );
}
