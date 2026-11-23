// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! Temporary agents (0020) through a real server, real git repositories, and
//! the real helper: create, start, objective, the end by every reason, the
//! keep rule, the cap, the idle limit, the hooks, and the `temp.*` requests.

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;
use std::time::{Duration, Instant};

use agent_infrastructure::persistence::SharedConnection;
use agent_infrastructure::services::find_program;
use hierarchy_application::common::interfaces::HierarchyStore;
use hierarchy_infrastructure::persistence::repositories::hierarchy_repository::HierarchyRepository;
use hierarchy_infrastructure::services::agent_places::AgentPlaces;
use office_harnessd::server::ServerOptions;
use serde_json::{Value, json};
use support::fake_cli::FakeClis;
use support::office_client::{Client, Office, error_text, wait_for};
use support::running_office::serve_again_with;
use tempfile::TempDir;

/// The real server binary that the fake CLIs run as the helper.
const HELPER: &str = env!("CARGO_BIN_EXE_office-harnessd");

/// The watcher looks every 50 ms and 5 idle minutes pass in 1.5 s. The idle
/// limit must stay well above the 400 ms quiet period of the test office, or
/// a loaded machine ends a temp before its Needs you item exists.
const TICK: Duration = Duration::from_millis(50);
const MINUTE: Duration = Duration::from_millis(300);
const IDLE_MINUTES: u32 = 5;
const IDLE_LIMIT: Duration = MINUTE.saturating_mul(IDLE_MINUTES);

/// The longest an end may take (spec SC-002).
const END_LIMIT: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(50);

/// Twice the quiet period of the fake office: a typed objective is echoed,
/// and the temp shows `waiting for input` again after it.
const SETTLE: Duration = Duration::from_millis(900);

/// A CLI tail that prints once, so its agent shows `waiting for input`.
const READY: &str = "echo ready";
/// A CLI tail that takes the objective and then writes nothing.
const TAKES_OBJECTIVE_THEN_SILENT: &str = "echo ready\nread line";

/// A CLI tail that keeps writing until `<records>/<flag>` exists, so its
/// agent stays `working` and takes no report.
fn busy_until(records: &Path, flag: &str) -> String {
    format!(
        "while [ ! -e {}/{flag} ]; do echo tick; sleep 0.1; done",
        records.display()
    )
}

/// A fake CLI body that hands its session's environment to
/// `<records>/<name>.env`, runs `rest`, and stays alive.
fn sender_body(records: &Path, name: &str, rest: &str) -> String {
    let folder = records.display();
    format!(
        "env > {folder}/{name}.part\nmv {folder}/{name}.part {folder}/{name}.env\n{rest}\nsleep 30\n"
    )
}

/// The tail of a CLI that echoes what the office types into it to
/// `<records>/<name>.typed`.
fn typing_tail(records: &Path, name: &str) -> String {
    format!(
        "echo ready\nstty raw echo\ncat > {}/{name}.typed",
        records.display()
    )
}

/// The call credential and server file of an agent's live session.
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
    call(sender, "report.send", &json!({"kind": kind, "text": text})).await
}

fn store_of(office: &Office) -> Arc<dyn HierarchyStore> {
    let connection =
        rusqlite::Connection::open(office.serving.office.config.database_path()).expect("database");
    Arc::new(HierarchyRepository::new(
        SharedConnection::new(connection).handle(),
        AgentPlaces::new(),
    ))
}

/// Gives an agent its team through the same store the server built, so the
/// members that it creates sit under it.
fn place_lead_with_team(office: &Office, lead_agent_id: &str, team_name: &str) {
    hierarchy_application::CreateTeamHandler::new(store_of(office))
        .handle(
            &hierarchy_application::commands::create_team::CreateTeamRequest {
                name: team_name.to_owned(),
                project_id: None,
                lead_agent_id: lead_agent_id.parse().expect("agent id"),
            },
        )
        .expect("team");
}

/// The real git with no user configuration, for the fixture's own commits.
fn git(folder: &Path, arguments: &[&str]) -> String {
    let home = folder.parent().expect("parent").join("home");
    fs::create_dir_all(&home).expect("home");
    let output = Command::new("git")
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "-C",
        ])
        .arg(folder)
        .args(arguments)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", &home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn commit_all(folder: &Path, message: &str) {
    git(folder, &["add", "--all"]);
    git(folder, &["commit", "--quiet", "-m", message]);
}

/// An office whose login `PATH` finds the real git, a repository with one
/// commit on `main`, and settings that let the tests create many temps.
struct Fixture {
    office: Office,
    client: Client,
    repository_folder: TempDir,
}

impl Fixture {
    async fn open() -> Self {
        Self::open_with_tick(TICK).await
    }

    async fn open_with_tick(tick: Duration) -> Self {
        let clis = FakeClis::new();
        let environment = std::env::vars().collect();
        let git_program =
            find_program("git", &environment, Path::new("/")).expect("git is installed");
        clis.add("git", &format!("exec '{}' \"$@\"", git_program.display()));
        let office = Office::open_with_options(clis, options(tick)).await;
        let client = office.client().await;
        let mut fixture = Self {
            office,
            client,
            repository_folder: tempfile::tempdir().expect("repository folder"),
        };
        let repository = fixture.repository();
        fs::create_dir_all(&repository).expect("repository folder");
        git(&repository, &["init", "--quiet", "--initial-branch=main"]);
        fs::write(repository.join("a.txt"), "a\n").expect("file");
        commit_all(&repository, "first");
        // 1440 idle minutes last 432 s here, so only the tests about the idle
        // limit see it; the other settings let a test create many temps.
        for (key, value) in [
            ("temp_idle_minutes", "1440"),
            ("temp_cap", "20"),
            ("limit_creates_per_agent", "100"),
        ] {
            fixture.client.set_setting(key, value).await;
        }
        fixture
    }

    fn repository(&self) -> PathBuf {
        self.repository_folder.path().join("repo")
    }

    fn records(&self) -> &Path {
        self.office.records_path()
    }

    /// A CLI that dumps its environment, runs `rest`, and stays alive.
    fn add_cli(&self, name: &str, rest: &str) {
        self.office
            .clis
            .add(name, &sender_body(self.records(), name, rest));
    }

    /// Creates a temp as the person, in its own worktree, and returns it.
    async fn person_temp(&mut self, name: &str, objective: &str) -> Value {
        let answer = self
            .client
            .ask(
                "agent.create",
                json!({
                    "name": name,
                    "command": name,
                    "working_folder": self.repository(),
                    "own_worktree": true,
                    "temp": true,
                    "objective": objective,
                }),
            )
            .await;
        assert_eq!(answer["ok"], true, "{answer}");
        answer["result"].clone()
    }

    /// As `person_temp`, with a CLI that runs `rest`.
    async fn person_temp_running(&mut self, name: &str, objective: &str, rest: &str) -> Value {
        self.add_cli(name, rest);
        self.person_temp(name, objective).await
    }

    /// A lead with a team whose CLI runs `rest`, started, with its call
    /// credential.
    async fn lead(&mut self, name: &str, rest: &str) -> (String, Sender) {
        self.add_cli(name, rest);
        let lead = self
            .client
            .create_agent(&self.office, name, name, None)
            .await;
        place_lead_with_team(&self.office, &lead, &format!("{name} team"));
        self.client.start(&lead).await;
        let sender = sender_of(self.records(), name).await;
        (lead, sender)
    }

    /// A lead that types what it receives into `<name>.typed`.
    async fn typing_lead(&mut self, name: &str) -> (String, Sender) {
        let tail = typing_tail(self.records(), name);
        self.lead(name, &tail).await
    }

    /// Creates a temp whose CLI prints `ready` and stays, as `creator` through
    /// the helper, in the repository.
    async fn agent_temp(&mut self, creator: &Sender, name: &str, objective: &str) -> Value {
        self.agent_temp_running(creator, name, objective, READY)
            .await
    }

    async fn agent_temp_running(
        &mut self,
        creator: &Sender,
        name: &str,
        objective: &str,
        rest: &str,
    ) -> Value {
        self.add_cli(name, rest);
        call(
            creator,
            "agent.create",
            &json!({
                "name": name,
                "command": name,
                "working_folder": self.repository(),
                "temp": true,
                "objective": objective,
            }),
        )
        .await
    }

    async fn temp_list(&mut self) -> Value {
        let answer = self.client.ask("temp.list", json!({})).await;
        assert_eq!(answer["ok"], true, "{answer}");
        answer["result"].clone()
    }

    /// Waits until `count` finished entries exist, newest first.
    async fn wait_finished(&mut self, count: usize, limit: Duration) -> Vec<Value> {
        let deadline = Instant::now() + limit;
        loop {
            let listed = self.temp_list().await;
            let finished = listed["finished"].as_array().expect("finished").clone();
            if finished.len() >= count {
                return finished;
            }
            assert!(
                Instant::now() < deadline,
                "only {} of {count} finished entries: {listed}",
                finished.len()
            );
            tokio::time::sleep(POLL).await;
        }
    }

    async fn temp_count(&mut self) -> usize {
        self.temp_list().await["temps"]
            .as_array()
            .expect("temps")
            .len()
    }

    async fn actions_of(&mut self, agent_id: &str) -> Vec<Value> {
        let actions = self
            .client
            .ask("agent.actions", json!({"agent_id": agent_id}))
            .await;
        assert_eq!(actions["ok"], true, "{actions}");
        actions["result"].as_array().expect("entries").clone()
    }

    fn branches(&self) -> String {
        git(&self.repository(), &["branch", "--list"])
    }
}

fn options(tick: Duration) -> ServerOptions {
    ServerOptions {
        temp_tick: tick,
        temp_minute: MINUTE,
        ..ServerOptions::default()
    }
}

fn id_of(agent: &Value) -> String {
    agent["id"].as_str().expect("id").to_owned()
}

fn folder_of(agent: &Value) -> PathBuf {
    PathBuf::from(agent["worktree"]["folder"].as_str().expect("folder"))
}

fn entry_named<'a>(finished: &'a [Value], name: &str) -> &'a Value {
    finished
        .iter()
        .find(|entry| entry["name"] == name)
        .unwrap_or_else(|| panic!("no finished entry named {name}: {finished:?}"))
}

fn typed_text(records: &Path, name: &str) -> String {
    fs::read_to_string(records.join(format!("{name}.typed"))).unwrap_or_default()
}

async fn wait_typed(records: &Path, name: &str, needle: &str) {
    wait_for("the typed text", || {
        typed_text(records, name).contains(needle).then_some(())
    })
    .await;
}

#[tokio::test]
async fn the_person_creates_a_temp_that_starts_in_its_own_worktree_and_gets_its_objective() {
    let mut fixture = Fixture::open().await;
    let objective = "write notes.txt";
    let records = fixture.records().to_owned();
    fixture.add_cli(
        "worker",
        &format!(
            "echo ready\nstty raw echo\ndd bs=1 count={} of={}/typed 2>/dev/null",
            objective.len() + 1,
            records.display()
        ),
    );

    let temp = fixture.person_temp("worker", objective).await;

    assert_eq!(
        temp["temp"],
        json!({"objective": objective, "creator": "user"})
    );
    assert!(folder_of(&temp).is_dir(), "{temp}");
    let typed = wait_for("the objective", || {
        fs::read(records.join("typed"))
            .ok()
            .filter(|bytes| bytes.len() == objective.len() + 1)
    })
    .await;
    assert_eq!(typed, format!("{objective}\r").into_bytes());
    let listed = fixture.temp_list().await;
    assert_eq!(listed["temps"].as_array().expect("temps").len(), 1);
    assert_eq!(listed["temps"][0]["creator_name"], "the person");
    assert_eq!(listed["temps"][0]["parent_name"], Value::Null);
    let actions = fixture.actions_of(&id_of(&temp)).await;
    let names: Vec<&str> = actions
        .iter()
        .filter_map(|entry| entry["action"].as_str())
        .collect();
    assert_eq!(names, ["create", "start", "directive"], "{actions:?}");
    assert_eq!(actions[2]["actor"], "user");

    // A done report that no parent receives ends the temp too.
    let sender = sender_of(&records, "worker").await;
    let answer = report(&sender, "done", "notes written").await;
    assert_eq!(answer["ok"], true, "{answer}");
    let finished = fixture.wait_finished(1, END_LIMIT).await;
    assert_eq!(finished[0]["reason"], "done");
    assert_eq!(
        finished[0]["report"],
        json!({"kind": "done", "text": "notes written"})
    );
    assert_eq!(finished[0]["kept_folder"], Value::Null);
    assert!(!folder_of(&temp).exists());
    assert!(!fixture.branches().contains("office-harness/worker"));
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn a_lead_creates_three_member_temps_and_each_done_report_reaches_the_lead_and_ends_its_temp()
{
    let mut fixture = Fixture::open().await;
    let (lead, lead_sender) = fixture.typing_lead("lead").await;
    let names = ["t1", "t2", "t3"];
    let mut senders = Vec::new();
    for name in names {
        let created = fixture.agent_temp(&lead_sender, name, "do one thing").await;
        assert_eq!(created["ok"], true, "{created}");
        assert_eq!(created["result"]["temp"]["creator"], json!(lead));
        senders.push(sender_of(fixture.records(), name).await);
    }
    for sender in &senders {
        let answer = report(sender, "done", "finished").await;
        assert_eq!(answer["ok"], true, "{answer}");
    }

    let finished = fixture.wait_finished(3, END_LIMIT).await;

    for name in names {
        let entry = entry_named(&finished, name);
        assert_eq!(entry["reason"], "done", "{entry}");
        assert_eq!(entry["report"]["text"], "finished");
        assert_eq!(entry["parent_name"], "lead");
        assert_eq!(entry["creator_name"], "lead");
        assert_eq!(entry["kept_folder"], Value::Null);
    }
    assert_eq!(fixture.temp_count().await, 0);
    let records = fixture.records().to_owned();
    wait_for("three report lines", || {
        (typed_text(&records, "lead")
            .matches("[report from ")
            .count()
            == 3)
            .then_some(())
    })
    .await;
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn a_temp_whose_cli_exits_with_code_1_ends_and_its_parent_hears_of_it() {
    let mut fixture = Fixture::open().await;
    let (_lead, lead_sender) = fixture.typing_lead("lead").await;
    let name = "crasher";
    fixture.add_cli(name, "exit 1");
    let created = call(
        &lead_sender,
        "agent.create",
        &json!({
            "name": name,
            "command": name,
            "working_folder": fixture.repository(),
            "temp": true,
            "objective": "fail",
        }),
    )
    .await;
    assert_eq!(created["ok"], true, "{created}");
    let crashed = id_of(&created["result"]);

    let finished = fixture.wait_finished(1, Duration::from_secs(5)).await;

    assert_eq!(finished[0]["reason"], "exited with code 1");
    let records = fixture.records().to_owned();
    wait_typed(
        &records,
        "lead",
        &format!("[temp ended: {name} ({crashed}): exited with code 1]"),
    )
    .await;
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn the_keep_rule_keeps_a_worktree_with_changes_or_commits_and_removes_a_clean_one() {
    let mut fixture = Fixture::open().await;
    let mut temps = Vec::new();
    for name in ["clean", "dirty", "committed", "both"] {
        fixture.add_cli(name, "");
        temps.push((name, fixture.person_temp(name, "keep rule").await));
    }
    let folder = |name: &str| folder_of(&temps.iter().find(|(n, _)| *n == name).expect("temp").1);
    fs::write(folder("dirty").join("u.txt"), "u\n").expect("file");
    fs::write(folder("committed").join("c.txt"), "c\n").expect("file");
    commit_all(&folder("committed"), "work");
    fs::write(folder("both").join("c.txt"), "c\n").expect("file");
    commit_all(&folder("both"), "work");
    fs::write(folder("both").join("u.txt"), "u\n").expect("file");

    for (_, temp) in &temps {
        let ended = fixture
            .client
            .ask("temp.end", json!({"agent_id": id_of(temp)}))
            .await;
        assert_eq!(ended["ok"], true, "{ended}");
    }

    let finished = fixture.wait_finished(4, END_LIMIT).await;
    let clean = entry_named(&finished, "clean");
    assert_eq!(clean["kept_folder"], Value::Null, "{clean}");
    assert_eq!(clean["kept_branch"], Value::Null, "{clean}");
    assert_eq!(clean["reason"], "ended by the person");
    assert!(!folder("clean").exists());
    assert!(!fixture.branches().contains("office-harness/clean"));
    for name in ["dirty", "committed", "both"] {
        let entry = entry_named(&finished, name);
        assert_eq!(entry["kept_folder"], json!(folder(name)), "{entry}");
        assert_eq!(entry["kept_branch"], format!("office-harness/{name}"));
        assert!(folder(name).is_dir(), "{name}");
        assert!(
            fixture
                .branches()
                .contains(&format!("office-harness/{name}"))
        );
    }
    fixture.office.serving.stop().await;
}

fn refusal_of(answer: &Value) -> (String, String) {
    let (code, message) = error_text(answer);
    (code.to_owned(), message.to_owned())
}

async fn notifications_of(client: &mut Client, event: &str) -> Vec<Value> {
    let answer = client.ask("notification.list", json!({})).await;
    assert_eq!(answer["ok"], true, "{answer}");
    answer["result"]
        .as_array()
        .expect("notifications")
        .iter()
        .filter(|row| row["event"] == event)
        .cloned()
        .collect()
}

/// The notification follows the finish record, so a single read can come
/// too early on a busy machine.
async fn wait_notifications(client: &mut Client, event: &str, count: usize) -> Vec<Value> {
    let deadline = Instant::now() + END_LIMIT;
    loop {
        let notified = notifications_of(client, event).await;
        if notified.len() == count {
            return notified;
        }
        assert!(Instant::now() < deadline, "{notified:?}");
        tokio::time::sleep(POLL).await;
    }
}

async fn has_needs_you_item(client: &mut Client, agent_id: &str) -> bool {
    let listed = client.ask("needs_you.list", json!({})).await;
    listed["result"]["items"]
        .as_array()
        .expect("items")
        .iter()
        .any(|item| item["agent_id"] == agent_id)
}

/// Waits until the objective of the temp was typed and the temp settled into
/// waiting again, so the Needs you item that the test dismisses is the last.
async fn wait_objective_taken(fixture: &mut Fixture, agent_id: &str) {
    let deadline = Instant::now() + END_LIMIT;
    loop {
        let listed = fixture.temp_list().await;
        let typed = listed["temps"]
            .as_array()
            .expect("temps")
            .iter()
            .any(|row| row["agent"]["id"] == agent_id && row["agent"]["task"].is_object());
        if typed {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the objective was never typed: {listed}"
        );
        tokio::time::sleep(POLL).await;
    }
    tokio::time::sleep(SETTLE).await;
}

async fn wait_needs_you_item(client: &mut Client, agent_id: &str) {
    let deadline = Instant::now() + END_LIMIT;
    while !has_needs_you_item(client, agent_id).await {
        assert!(
            Instant::now() < deadline,
            "no Needs you item for {agent_id}"
        );
        tokio::time::sleep(POLL).await;
    }
}

#[tokio::test]
async fn the_cap_refuses_the_next_temp_for_an_agent_and_the_person_and_holds_nobody() {
    let mut fixture = Fixture::open().await;
    fixture.client.set_setting("temp_cap", "3").await;
    let (lead, sender) = fixture.lead("lead", READY).await;
    for name in ["t1", "t2", "t3"] {
        let created = fixture.agent_temp(&sender, name, "job").await;
        assert_eq!(created["ok"], true, "{created}");
    }
    let cap_text = "the temp cap is 3; end a temp or raise the cap";

    for round in 0..6 {
        let refused = fixture
            .agent_temp(&sender, &format!("over-{round}"), "job")
            .await;
        assert_eq!(
            refusal_of(&refused),
            ("refused".to_owned(), cap_text.to_owned())
        );
    }
    fixture.add_cli("by-person", READY);
    let by_person = fixture
        .client
        .ask(
            "agent.create",
            json!({
                "name": "by-person",
                "command": "by-person",
                "working_folder": fixture.repository(),
                "temp": true,
                "objective": "job",
            }),
        )
        .await;
    assert_eq!(
        refusal_of(&by_person),
        ("refused".to_owned(), cap_text.to_owned())
    );

    // Six cap refusals are no rate-limit refusals: nobody is on hold, and the
    // lead still creates an agent that is not a temp.
    let holds = fixture.client.ask("hold.list", json!({})).await;
    assert_eq!(holds["result"]["agents"], json!([]), "{holds}");
    assert_eq!(holds["result"]["office"], Value::Null, "{holds}");
    let plain = call(
        &sender,
        "agent.create",
        &json!({"name": "plain", "command": "plain", "working_folder": fixture.repository()}),
    )
    .await;
    assert_eq!(plain["ok"], true, "{plain}");
    let actions = fixture.actions_of(&lead).await;
    let recorded = actions
        .iter()
        .filter(|entry| entry["action"] == "refused")
        .filter(|entry| {
            entry["result"]
                .as_str()
                .is_some_and(|text| text.contains(cap_text))
        })
        .count();
    assert_eq!(recorded, 6, "{actions:?}");
    assert_eq!(fixture.temp_count().await, 3);
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn a_silent_temp_ends_for_the_idle_limit_and_a_busy_one_and_one_under_a_hold_stay() {
    let mut fixture = Fixture::open().await;
    fixture
        .client
        .set_setting("temp_idle_minutes", &IDLE_MINUTES.to_string())
        .await;
    let (_lead, sender) = fixture.typing_lead("lead").await;
    let quiet = fixture
        .agent_temp_running(&sender, "quiet", "stay quiet", TAKES_OBJECTIVE_THEN_SILENT)
        .await;
    let held = fixture
        .agent_temp_running(&sender, "held", "stay quiet", TAKES_OBJECTIVE_THEN_SILENT)
        .await;
    let busy_tail =
        format!("{TAKES_OBJECTIVE_THEN_SILENT}\nwhile true; do echo tick; sleep 0.1; done");
    let busy = fixture
        .agent_temp_running(&sender, "busy", "keep writing", &busy_tail)
        .await;
    let (quiet, held, busy) = (
        id_of(&quiet["result"]),
        id_of(&held["result"]),
        id_of(&busy["result"]),
    );
    // Every new `waiting for input` brings a Needs you item, which keeps a
    // temp from the idle end until the person dismisses it.
    for temp in [&quiet, &held, &busy] {
        wait_objective_taken(&mut fixture, temp).await;
    }
    let on_hold = fixture
        .client
        .ask("hold.set", json!({"agent_id": held}))
        .await;
    assert_eq!(on_hold["ok"], true, "{on_hold}");
    fixture.client.dismiss_needs_you(&quiet).await;
    fixture.client.dismiss_needs_you(&held).await;

    let finished = fixture.wait_finished(1, END_LIMIT).await;

    assert_eq!(finished[0]["name"], "quiet");
    assert_eq!(finished[0]["reason"], "no output for 5 minutes");
    let notice = format!("[temp ended: quiet ({quiet}): no output for 5 minutes]");
    wait_typed(fixture.records(), "lead", &notice).await;
    // The parent heard of it, so the person gets no notification.
    assert_eq!(
        notifications_of(&mut fixture.client, "temp_ended")
            .await
            .len(),
        0
    );
    tokio::time::sleep(IDLE_LIMIT * 3).await;
    let listed = fixture.temp_list().await;
    let remaining: Vec<&Value> = listed["temps"]
        .as_array()
        .expect("temps")
        .iter()
        .map(|row| &row["agent"]["id"])
        .collect();
    assert_eq!(remaining.len(), 2, "{listed}");
    assert!(remaining.contains(&&json!(held)) && remaining.contains(&&json!(busy)));
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn a_temp_with_a_needs_you_item_is_not_ended_and_ends_one_idle_limit_after_the_dismiss() {
    let mut fixture = Fixture::open().await;
    fixture
        .client
        .set_setting("temp_idle_minutes", &IDLE_MINUTES.to_string())
        .await;
    let temp = fixture
        .person_temp_running("paused", "wait", TAKES_OBJECTIVE_THEN_SILENT)
        .await;
    let temp = id_of(&temp);
    wait_needs_you_item(&mut fixture.client, &temp).await;

    tokio::time::sleep(IDLE_LIMIT * 3).await;

    assert_eq!(fixture.temp_count().await, 1, "the item keeps the temp");
    let dismissed = fixture
        .client
        .ask("needs_you.dismiss", json!({"agent_id": temp}))
        .await;
    assert_eq!(dismissed["ok"], true, "{dismissed}");
    let dismissed_at = Instant::now();
    let finished = fixture.wait_finished(1, END_LIMIT).await;
    assert!(
        dismissed_at.elapsed() >= IDLE_LIMIT / 2,
        "{:?}",
        dismissed_at.elapsed()
    );
    assert_eq!(finished[0]["reason"], "no output for 5 minutes");
    // No parent heard of it, so the person gets exactly one notification.
    let notified = wait_notifications(&mut fixture.client, "temp_ended", 1).await;
    assert_eq!(notified[0]["agent_name"], "paused");
    assert_eq!(notified[0]["detail"], "no output for 5 minutes");
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn deleting_a_lead_ends_its_three_temps_and_keeps_their_worktrees() {
    let mut fixture = Fixture::open().await;
    let (lead, sender) = fixture.lead("L1", READY).await;
    let mut folders = Vec::new();
    for name in ["m1", "m2", "m3"] {
        let created = fixture.agent_temp(&sender, name, "job").await;
        assert_eq!(created["ok"], true, "{created}");
        folders.push(folder_of(&created["result"]));
    }

    let deleted = fixture
        .client
        .ask(
            "agent.delete",
            json!({"agent_id": lead, "include_descendants": true, "descendant_count": 3}),
        )
        .await;

    assert_eq!(deleted["ok"], true, "{deleted}");
    let finished = fixture.wait_finished(3, END_LIMIT).await;
    assert_eq!(finished.len(), 3, "the lead is no temp: {finished:?}");
    for (name, folder) in ["m1", "m2", "m3"].iter().zip(&folders) {
        let entry = entry_named(&finished, name);
        assert_eq!(entry["reason"], "its parent L1 was deleted", "{entry}");
        assert_eq!(entry["kept_folder"], json!(folder));
        assert!(folder.is_dir());
        let actions = fixture
            .actions_of(&id_of(&json!({"id": entry["agent_id"]})))
            .await;
        let ended = actions
            .iter()
            .find(|row| row["action"] == "end")
            .expect("end entry");
        assert_eq!(ended["actor"], "user");
        assert_eq!(ended["detail"], "its parent L1 was deleted");
    }
    assert_eq!(fixture.temp_count().await, 0);
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn deleting_one_temp_leaves_a_finished_entry_naming_the_person() {
    let mut fixture = Fixture::open().await;
    let temp = fixture.person_temp_running("solo", "job", READY).await;

    let deleted = fixture
        .client
        .ask("agent.delete", json!({"agent_id": id_of(&temp)}))
        .await;

    assert_eq!(deleted["ok"], true, "{deleted}");
    let finished = fixture.wait_finished(1, END_LIMIT).await;
    assert_eq!(finished[0]["reason"], "deleted by the person");
    assert_eq!(finished[0]["kept_folder"], json!(folder_of(&temp)));
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn a_lead_temps_done_report_ends_its_two_member_temps_first() {
    let mut fixture = Fixture::open().await;
    let scout = fixture
        .client
        .create_agent(&fixture.office, "scout", "scout", None)
        .await;
    let team = fixture
        .client
        .ask(
            "team.create",
            json!({"name": "LT team", "lead_agent_id": scout}),
        )
        .await;
    assert_eq!(team["ok"], true, "{team}");
    let deleted = fixture
        .client
        .ask("agent.delete", json!({"agent_id": scout}))
        .await;
    assert_eq!(deleted["ok"], true, "{deleted}");
    fixture.add_cli("LT", READY);
    let lead_temp = fixture
        .client
        .ask(
            "agent.create",
            json!({
                "name": "LT",
                "command": "LT",
                "working_folder": fixture.repository(),
                "own_worktree": true,
                "role": {"kind": "lead", "team_id": team["result"]["id"]},
                "temp": true,
                "objective": "lead the job",
            }),
        )
        .await;
    assert_eq!(lead_temp["ok"], true, "{lead_temp}");
    let lead_temp = id_of(&lead_temp["result"]);
    let lt = sender_of(fixture.records(), "LT").await;
    let mut members = Vec::new();
    for name in ["m1", "m2"] {
        fixture.add_cli(name, READY);
        // No temp choice: a temp that creates an agent makes a temp.
        let created = call(
            &lt,
            "agent.create",
            &json!({
                "name": name,
                "command": name,
                "working_folder": fixture.repository(),
                "objective": "sub job",
            }),
        )
        .await;
        assert_eq!(created["ok"], true, "{created}");
        assert_eq!(created["result"]["temp"]["creator"], json!(lead_temp));
        members.push(id_of(&created["result"]));
    }

    let answer = report(&lt, "done", "all done").await;
    assert_eq!(answer["ok"], true, "{answer}");

    let finished = fixture.wait_finished(3, END_LIMIT).await;
    assert_eq!(finished[0]["name"], "LT", "newest first: {finished:?}");
    assert_eq!(finished[0]["reason"], "done");
    let lead_ended = end_time(&mut fixture, &lead_temp).await;
    for member in &members {
        let entry = finished
            .iter()
            .find(|entry| entry["agent_id"] == json!(member))
            .expect("member entry");
        assert_eq!(entry["reason"], "its parent LT ended", "{entry}");
        assert!(end_time(&mut fixture, member).await <= lead_ended);
    }
    fixture.office.serving.stop().await;
}

/// The time of the `end` entry in an agent's record.
async fn end_time(fixture: &mut Fixture, agent_id: &str) -> i64 {
    let actions = fixture.actions_of(agent_id).await;
    actions
        .iter()
        .find(|entry| entry["action"] == "end")
        .and_then(|entry| entry["at"].as_i64())
        .expect("an end entry")
}

#[tokio::test]
async fn five_entries_survive_a_restart_a_dismiss_removes_one_and_an_interrupted_temp_stays() {
    let mut fixture = Fixture::open().await;
    let mut kept_folder = PathBuf::new();
    for index in 0..5 {
        let name = format!("e{index}");
        let temp = fixture.person_temp_running(&name, "job", READY).await;
        if index == 0 {
            kept_folder = folder_of(&temp);
            fs::write(kept_folder.join("u.txt"), "u\n").expect("file");
        }
        let ended = fixture
            .client
            .ask("temp.end", json!({"agent_id": id_of(&temp)}))
            .await;
        assert_eq!(ended["ok"], true, "{ended}");
    }
    let survivor = fixture.person_temp_running("survivor", "job", READY).await;
    let survivor = id_of(&survivor);
    fixture
        .client
        .wait_for_status(&survivor, "waiting_for_input")
        .await;

    let stopped = fixture.office.serving.stop().await;
    fixture.office.serving =
        serve_again_with(stopped, fixture.office.clis.login_setup(), options(TICK)).await;
    fixture.client = fixture.office.client().await;

    let listed = fixture.temp_list().await;
    assert_eq!(listed["finished"].as_array().expect("finished").len(), 5);
    tokio::time::sleep(POLL * 20).await;
    let listed = fixture.temp_list().await;
    assert_eq!(
        listed["finished"].as_array().expect("finished").len(),
        5,
        "an interrupted temp is no end"
    );
    assert_eq!(listed["temps"][0]["agent"]["status"], "interrupted");
    assert_eq!(listed["temps"][0]["quiet_seconds"], Value::Null);
    let first = entry_named(listed["finished"].as_array().expect("finished"), "e0");
    let entry_id = first["id"].as_str().expect("entry id").to_owned();

    let dismissed = fixture
        .client
        .ask("temp.dismiss", json!({"entry_id": entry_id}))
        .await;

    assert_eq!(dismissed["ok"], true, "{dismissed}");
    let listed = fixture.temp_list().await;
    assert_eq!(listed["finished"].as_array().expect("finished").len(), 4);
    assert!(kept_folder.is_dir(), "a dismiss removes no folder");
    assert!(fixture.branches().contains("office-harness/e0"));
    let first_agent = first["agent_id"].as_str().expect("agent id").to_owned();
    let actions = fixture.actions_of(&first_agent).await;
    let dismiss = actions
        .iter()
        .find(|entry| entry["action"] == "dismiss")
        .expect("a dismiss entry");
    assert_eq!(dismiss["actor"], "user", "{dismiss}");
    assert_eq!(dismiss["detail"], json!(entry_id), "{dismiss}");
    let again = fixture
        .client
        .ask("temp.dismiss", json!({"entry_id": entry_id}))
        .await;
    assert_eq!(refusal_of(&again).0, "not_found", "{again}");
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn temp_list_shows_ten_temps_at_several_places_with_their_creator_and_parent() {
    let mut fixture = Fixture::open().await;
    let (_l1, first) = fixture.lead("L1", READY).await;
    let (_l2, second) = fixture.lead("L2", READY).await;
    for index in 0..4 {
        fixture
            .person_temp_running(&format!("p{index}"), "person job", READY)
            .await;
    }
    for (creator, names) in [(&first, ["a1", "a2", "a3"]), (&second, ["b1", "b2", "b3"])] {
        for name in names {
            let created = fixture.agent_temp(creator, name, "lead job").await;
            assert_eq!(created["ok"], true, "{created}");
        }
    }

    let deadline = Instant::now() + END_LIMIT;
    let listed = loop {
        let listed = fixture.temp_list().await;
        let rows = listed["temps"].as_array().expect("temps");
        if rows.len() == 10 && rows.iter().all(|row| row["quiet_seconds"].is_number()) {
            break listed;
        }
        assert!(Instant::now() < deadline, "{listed}");
        tokio::time::sleep(POLL).await;
    };

    for row in listed["temps"].as_array().expect("temps") {
        let name = row["agent"]["name"].as_str().expect("name");
        let (creator, parent) = match &name[..1] {
            "p" => ("the person", Value::Null),
            "a" => ("L1", json!("L1")),
            _ => ("L2", json!("L2")),
        };
        assert_eq!(row["creator_name"], creator, "{row}");
        assert_eq!(row["parent_name"], parent, "{row}");
        assert!(row["agent"]["temp"]["objective"].is_string(), "{row}");
        assert!(row["run_seconds"].is_number(), "{row}");
    }
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn a_done_report_waiting_in_a_working_parents_queue_is_typed_after_the_temp_ended() {
    let mut fixture = Fixture::open().await;
    let tail = format!(
        "{}\n{}",
        busy_until(fixture.records(), "go"),
        typing_tail(fixture.records(), "lead")
    );
    let (lead, sender) = fixture.lead("lead", &tail).await;
    fixture.client.wait_for_status(&lead, "working").await;
    let created = fixture.agent_temp(&sender, "late", "job").await;
    let late = id_of(&created["result"]);
    let temp = sender_of(fixture.records(), "late").await;

    let answer = report(&temp, "done", "all green").await;

    assert_eq!(answer["result"]["delivery"], "waiting", "{answer}");
    fixture.wait_finished(1, END_LIMIT).await;
    assert_eq!(fixture.temp_count().await, 0);
    assert_eq!(typed_text(fixture.records(), "lead"), "");
    fs::write(fixture.records().join("go"), "").expect("flag");
    wait_typed(
        fixture.records(),
        "lead",
        &format!("[report from late ({late}): done] all green"),
    )
    .await;
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn new_tasks_and_continued_sessions_on_running_temps_end_no_temp() {
    let mut fixture = Fixture::open_with_tick(Duration::from_millis(10)).await;
    // Only a claude command can continue a session.
    fixture.add_cli("claude", READY);
    let mut temps = Vec::new();
    for name in ["s1", "s2"] {
        let created = fixture
            .client
            .ask(
                "agent.create",
                json!({
                    "name": name,
                    "command": "claude",
                    "working_folder": fixture.repository(),
                    "own_worktree": true,
                    "temp": true,
                    "objective": "job",
                }),
            )
            .await;
        assert_eq!(created["ok"], true, "{created}");
        temps.push(id_of(&created["result"]));
    }
    for id in &temps {
        fixture
            .client
            .wait_for_status(id, "waiting_for_input")
            .await;
    }

    for round in 0..10 {
        let id = &temps[round % temps.len()];
        let started = fixture
            .client
            .ask("agent.new_task", json!({"agent_id": id}))
            .await;
        assert_eq!(started["ok"], true, "{started}");
    }
    for round in 0..10 {
        let id = &temps[round % temps.len()];
        // The newest session runs; the one before it ended and continues.
        let previous = fixture.client.sessions(id).await[1]["id"]
            .as_str()
            .expect("session id")
            .to_owned();
        let continued = fixture
            .client
            .ask("session.continue", json!({"session_id": previous}))
            .await;
        assert_eq!(continued["ok"], true, "{continued}");
    }

    tokio::time::sleep(POLL * 6).await;
    let listed = fixture.temp_list().await;
    assert_eq!(
        listed["temps"].as_array().expect("temps").len(),
        2,
        "{listed}"
    );
    assert_eq!(listed["finished"], json!([]), "{listed}");
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn a_temp_that_ends_refuses_a_new_task_and_a_second_end() {
    let mut fixture = Fixture::open().await;
    fixture.client.set_setting("stop_grace_ms", "2000").await;
    let temp = fixture
        .person_temp_running(
            "stubborn",
            "job",
            "trap '' INT TERM HUP\necho ready\nwhile true; do sleep 0.2; done",
        )
        .await;
    let temp = id_of(&temp);
    fixture
        .client
        .wait_for_status(&temp, "waiting_for_input")
        .await;
    // The CLI ignores Ctrl+C, so the stop of the end takes the grace time.
    let ending = fixture
        .client
        .send("temp.end", json!({"agent_id": temp}))
        .await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut other = fixture.office.client().await;

    let second = other.ask("temp.end", json!({"agent_id": temp})).await;
    let task = other
        .send("agent.new_task", json!({"agent_id": temp}))
        .await;

    assert_eq!(
        refusal_of(&second),
        ("conflict".to_owned(), "the temp already ended".to_owned())
    );
    // The task waits for the status order, which the stop of the end holds;
    // when it gets its turn the temp is still claimed.
    let refused = other.answer_of(task).await;
    assert_eq!(
        refusal_of(&refused),
        ("conflict".to_owned(), "the temp is ending".to_owned())
    );
    let ended = fixture.client.answer_of(ending).await;
    assert_eq!(ended["ok"], true, "{ended}");
    assert_eq!(fixture.temp_count().await, 0);
    let again = fixture
        .client
        .ask("temp.end", json!({"agent_id": temp}))
        .await;
    assert_eq!(
        refusal_of(&again),
        ("conflict".to_owned(), "the temp already ended".to_owned())
    );
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn a_stop_by_the_parent_or_the_person_ends_the_temp_naming_the_actor() {
    let mut fixture = Fixture::open().await;
    let (_lead, sender) = fixture.lead("Boss", READY).await;
    let by_parent = fixture.agent_temp(&sender, "p1", "job").await;
    let by_person = fixture.agent_temp(&sender, "p2", "job").await;

    let stopped = call(
        &sender,
        "agent.stop",
        &json!({"agent_id": id_of(&by_parent["result"])}),
    )
    .await;
    assert_eq!(stopped["ok"], true, "{stopped}");
    let stopped = fixture
        .client
        .ask(
            "agent.stop",
            json!({"agent_id": id_of(&by_person["result"])}),
        )
        .await;
    assert_eq!(stopped["ok"], true, "{stopped}");

    let finished = fixture.wait_finished(2, END_LIMIT).await;
    assert_eq!(entry_named(&finished, "p1")["reason"], "stopped by Boss");
    assert_eq!(
        entry_named(&finished, "p2")["reason"],
        "stopped by the person"
    );
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn the_create_rules_of_a_temp_and_the_temp_requests_refuse_what_they_must() {
    let mut fixture = Fixture::open().await;
    let (_lead, lead) = fixture.lead("lead", READY).await;
    let member = fixture.agent_temp(&lead, "m", "job").await;
    assert_eq!(member["ok"], true, "{member}");
    assert!(
        member["result"]["worktree"].is_object(),
        "own worktree by default"
    );
    let member_sender = sender_of(fixture.records(), "m").await;

    // A member temp creates no agent (0002 refuses a member's create).
    let refused = call(
        &member_sender,
        "agent.create",
        &json!({"name": "x", "command": "x", "working_folder": fixture.repository(), "objective": "job"}),
    )
    .await;
    assert_eq!(refusal_of(&refused).0, "refused", "{refused}");
    // An own worktree belongs to a temp only.
    let plain = call(
        &lead,
        "agent.create",
        &json!({"name": "plain", "command": "plain", "working_folder": fixture.repository(), "own_worktree": true}),
    )
    .await;
    assert_eq!(
        refusal_of(&plain),
        (
            "invalid".to_owned(),
            "own_worktree is only for a temp".to_owned()
        )
    );
    // A temp needs its objective.
    let blank = call(
        &lead,
        "agent.create",
        &json!({"name": "blank", "command": "blank", "working_folder": fixture.repository(), "temp": true}),
    )
    .await;
    assert_eq!(
        refusal_of(&blank),
        ("invalid".to_owned(), "a temp needs an objective".to_owned())
    );
    // Turning the worktree off makes a temp without one.
    fixture.add_cli("bare", READY);
    let bare = call(
        &lead,
        "agent.create",
        &json!({"name": "bare", "command": "bare", "working_folder": fixture.repository(), "temp": true, "objective": "job", "own_worktree": false}),
    )
    .await;
    assert_eq!(bare["result"]["worktree"], Value::Null, "{bare}");
    // No agent reaches `temp.*`.
    for kind in ["temp.list", "temp.end", "temp.dismiss", "temp.end_check"] {
        let answer = call(&lead, kind, &json!({})).await;
        assert_eq!(refusal_of(&answer).0, "refused", "{kind}: {answer}");
    }
    // The person may not make the office-only check.
    let check = fixture
        .client
        .ask(
            "temp.end_check",
            json!({"agent_id": id_of(&member["result"]), "seen": "stopped"}),
        )
        .await;
    assert_eq!(
        refusal_of(&check),
        (
            "invalid".to_owned(),
            "temp.end_check is made by the office only".to_owned()
        )
    );
    let plain_agent = fixture
        .client
        .create_agent(&fixture.office, "Plain", "plain", None)
        .await;
    let not_temp = fixture
        .client
        .ask("temp.end", json!({"agent_id": plain_agent}))
        .await;
    assert_eq!(
        refusal_of(&not_temp),
        ("invalid".to_owned(), "the agent is not a temp".to_owned())
    );
    let unknown = fixture
        .client
        .ask(
            "temp.end",
            json!({"agent_id": "00000000-0000-4000-8000-000000000001"}),
        )
        .await;
    assert_eq!(refusal_of(&unknown).0, "not_found", "{unknown}");
    let extra = fixture.client.ask("temp.list", json!({"stray": 1})).await;
    assert_eq!(
        refusal_of(&extra),
        (
            "invalid".to_owned(),
            "stray is not a field of temp.list".to_owned()
        )
    );
    fixture.office.serving.stop().await;
}
