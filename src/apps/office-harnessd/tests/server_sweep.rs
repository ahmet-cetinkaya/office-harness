// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]
#![cfg(target_os = "linux")]

//! The orphan sweep of 0017 (T3) against real server processes that get a
//! SIGKILL. The stand-in CLIs ignore the hang-up, so they outlive the server
//! like the CLI of research R1 of 0013. Each helper starts in a session of its
//! own. The test reads process start times from `/proc`, so it runs on Linux
//! only.

mod support;

use std::fs;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use office_harnessd::config::Config;
use rusqlite::Connection;
use serde_json::{Value, json};
use support::config_in;
use support::fake_cli::FakeClis;
use support::office_client::{Client, recording_body, wait_for, wait_for_records};
use support::server_process::{ServerProcess, connect, spawn_server, wait_until_started};

/// The bound of SC-003: with 30 `lost` sessions and the default grace period
/// the list shows within 30 seconds of the server start.
const SC003_BOUND: Duration = Duration::from_secs(30);
const AGENT_COUNT: usize = 30;
const STOP_CYCLES: usize = 10;
const POLL: Duration = Duration::from_millis(50);
/// The grace period of the tests that wait for it: long enough to look at
/// the running sweep, short enough to keep the suite fast.
const SHORT_GRACE_MS: &str = "1000";
const LONG_GRACE_MS: &str = "3000";
const PROCESS_STATE_FIELD_AFTER_NAME: usize = 0;
const START_TIME_FIELD_AFTER_NAME: usize = 19;
const ZOMBIE_STATE: &str = "Z";

/// A CLI that ignores the hang-up and the request to end, and has a helper
/// in a session of its own that does the same.
const STUBBORN_CLI: &str = "stubborn-cli";
/// As the stubborn CLI, but it ends on the request, and so does its helper.
const PLAIN_CLI: &str = "plain-cli";
/// A CLI that ends with the hang-up of the server's death.
const QUICK_CLI: &str = "quick-cli";

/// One process as the test saw it: a reused process id does not count.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Mark {
    pid: u32,
    start_time: String,
}

impl Mark {
    fn read(pid: u32) -> Option<Self> {
        let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let after_name = stat.rsplit_once(')')?.1;
        let fields: Vec<&str> = after_name.split_whitespace().collect();
        if fields.get(PROCESS_STATE_FIELD_AFTER_NAME) == Some(&ZOMBIE_STATE) {
            return None;
        }
        Some(Self {
            pid,
            start_time: (*fields.get(START_TIME_FIELD_AFTER_NAME)?).to_owned(),
        })
    }

    fn still_runs(&self) -> bool {
        Self::read(self.pid).as_ref() == Some(self)
    }

    fn kill(&self) {
        if self.still_runs() {
            let _ = Command::new("kill")
                .args(["-KILL", &self.pid.to_string()])
                .status();
        }
    }
}

/// The folders, the stand-in CLIs, and every process the test saw, which end
/// when the test ends, also on a failed assertion.
struct Scenario {
    _folder: tempfile::TempDir,
    _work: tempfile::TempDir,
    records: tempfile::TempDir,
    work_path: PathBuf,
    config: Config,
    clis: FakeClis,
    marks: Vec<Mark>,
    decoys: Vec<Child>,
}

impl Drop for Scenario {
    fn drop(&mut self) {
        for mark in &self.marks {
            mark.kill();
        }
        for decoy in &mut self.decoys {
            let _ = decoy.kill();
            let _ = decoy.wait();
        }
    }
}

impl Scenario {
    fn new() -> Self {
        let folder = tempfile::tempdir().expect("folder");
        let work = tempfile::tempdir().expect("work folder");
        let records = tempfile::tempdir().expect("records");
        let config = config_in(&folder);
        let clis = FakeClis::new();
        let _ = clis.login_setup();
        let scenario = Self {
            work_path: work.path().to_path_buf(),
            _folder: folder,
            _work: work,
            records,
            config,
            clis,
            marks: Vec::new(),
            decoys: Vec::new(),
        };
        scenario.add_clis();
        scenario
    }

    fn add_clis(&self) {
        let records = self.records.path().display();
        let helper = format!(
            "setsid sh -c 'printf \"%s\\n\" \"$$\" >> {records}/helper-pids; exec sleep 600' &"
        );
        let record_cli = format!("printf '%s\\n' \"$$\" >> {records}/cli-pids");
        self.clis.add(
            STUBBORN_CLI,
            &format!("trap '' HUP TERM\n{record_cli}\n{helper}\necho ready\nwait"),
        );
        self.clis.add(
            PLAIN_CLI,
            &format!("trap '' HUP\n{record_cli}\n{helper}\necho ready\nwait"),
        );
        self.clis.add(
            QUICK_CLI,
            &format!("{record_cli}\necho ready\nexec sleep 600"),
        );
        self.clis.add(
            "claude",
            &recording_body(self.records.path(), "echo ready\nexec sleep 600"),
        );
    }

    fn spawn(&self) -> ServerProcess {
        spawn_server(&self.config, &self.clis)
    }

    async fn client(&self) -> Client {
        connect(&self.config).await
    }

    /// Creates an agent that runs `command` and starts it. Returns the agent
    /// id and the session id.
    async fn create_and_start(
        &self,
        client: &mut Client,
        name: &str,
        command: &str,
    ) -> (String, String) {
        let agent = self.create(client, name, command).await;
        let session = client.start(&agent).await;
        (
            agent,
            session["id"].as_str().expect("session id").to_owned(),
        )
    }

    async fn create(&self, client: &mut Client, name: &str, command: &str) -> String {
        let created = client
            .ask(
                "agent.create",
                json!({"name": name, "command": command, "working_folder": self.work_path}),
            )
            .await;
        assert_eq!(created["ok"], true, "{created}");
        created["result"]["id"].as_str().expect("id").to_owned()
    }

    fn database(&self) -> Connection {
        Connection::open(self.config.database_path()).expect("database")
    }

    fn pids_in(&self, name: &str) -> Vec<u32> {
        fs::read_to_string(self.records.path().join(name))
            .unwrap_or_default()
            .lines()
            .filter_map(|line| line.trim().parse().ok())
            .collect()
    }

    /// Waits until `count` stand-in CLIs and as many helpers have written
    /// their pids and the office stored an open identity for each of those
    /// processes, then remembers them for the cleanup.
    async fn wait_for_stored_processes(&mut self, count: usize) -> Vec<Mark> {
        let pids = wait_for("the pids of the stand-ins", || {
            let mut pids = self.pids_in("cli-pids");
            let clis = pids.len();
            pids.extend(self.pids_in("helper-pids"));
            (clis >= count && pids.len() >= count * 2).then_some(pids)
        })
        .await;
        let listed: Vec<String> = pids.iter().map(u32::to_string).collect();
        let open_rows = format!(
            "SELECT COUNT(*) FROM agent_session_processes \
             WHERE state = 'open' AND pid IN ({})",
            listed.join(", ")
        );
        wait_for("the stored identities of the stand-ins", || {
            let stored: i64 = self
                .database()
                .query_row(&open_rows, [], |row| row.get(0))
                .ok()?;
            (usize::try_from(stored).ok()? >= pids.len()).then_some(())
        })
        .await;
        let marks: Vec<Mark> = pids
            .into_iter()
            .map(|pid| Mark::read(pid).expect("a stand-in runs"))
            .collect();
        self.marks.extend(marks.iter().cloned());
        marks
    }

    /// The `end_leftover` entries: the actor, the agent, and the detail.
    fn sweep_entries(&self) -> Vec<SweepEntry> {
        let database = self.database();
        let mut statement = database
            .prepare(
                "SELECT actor, target_agent_id, result, detail FROM audit_actions \
                 WHERE action = 'end_leftover' ORDER BY id",
            )
            .expect("prepare");
        statement
            .query_map([], |row| {
                Ok(SweepEntry {
                    actor: row.get(0)?,
                    agent: row.get(1)?,
                    result: row.get(2)?,
                    detail: row.get(3)?,
                })
            })
            .expect("query")
            .collect::<Result<_, _>>()
            .expect("entries")
    }

    fn entry_for(&self, pid: u32) -> SweepEntry {
        let mut found: Vec<SweepEntry> = self
            .sweep_entries()
            .into_iter()
            .filter(|entry| entry.detail.contains(&format!(": pid {pid}, ")))
            .collect();
        assert_eq!(found.len(), 1, "one entry for pid {pid}: {found:?}");
        found.remove(0)
    }
}

#[derive(Clone, Debug)]
struct SweepEntry {
    actor: String,
    agent: Option<String>,
    result: String,
    detail: String,
}

/// Ends the server with a SIGKILL, as a crash does.
fn kill_server(server: &mut ServerProcess) {
    server.0.kill().expect("SIGKILL the server");
    server.0.wait().expect("reap the server");
}

async fn set_grace(client: &mut Client, milliseconds: &str) {
    let set = client
        .ask(
            "settings.set",
            json!({"key": "stop_grace_ms", "value": milliseconds}),
        )
        .await;
    assert_eq!(set["ok"], true, "{set}");
}

/// Asks for the plan until the sweep is over and returns the answer. While it
/// runs, the answer holds no agent (FR-007).
async fn plan_after_the_sweep(client: &mut Client, limit: Duration) -> Value {
    let deadline = Instant::now() + limit;
    loop {
        let plan = client.ask("resume.plan", json!({})).await;
        assert_eq!(plan["ok"], true, "{plan}");
        if plan["result"]["sweeping"] == false {
            return plan["result"].clone();
        }
        assert_eq!(plan["result"]["agents"], json!([]), "{plan}");
        assert!(Instant::now() < deadline, "the sweep took over {limit:?}");
        tokio::time::sleep(POLL).await;
    }
}

/// A stubborn CLI waits for its helper, so it can end by itself when the
/// sweep forces the helper first; the entry then says "ended". Either way the
/// entry is a successful end of the process, never a "left" or a failure.
fn assert_ended_by_the_sweep(entry: &SweepEntry) {
    assert_eq!(entry.result, "ok", "{entry:?}");
    assert!(
        entry.detail.contains(": ended by force: pid ") || entry.detail.contains(": ended: pid "),
        "{entry:?}"
    );
}

fn forced_count(entries: &[SweepEntry]) -> usize {
    entries
        .iter()
        .filter(|entry| entry.detail.contains(": ended by force: pid "))
        .count()
}

fn assert_all_gone(marks: &[Mark]) {
    let running: Vec<_> = marks.iter().filter(|mark| mark.still_runs()).collect();
    assert!(running.is_empty(), "still running: {running:?}");
}

#[tokio::test]
async fn thirty_stubborn_agents_with_helpers_end_by_force_before_the_plan_lists_them() {
    let mut scenario = Scenario::new();
    let mut server = scenario.spawn();
    wait_until_started(&scenario.config, 1).await;
    let mut client = scenario.client().await;
    for number in 0..AGENT_COUNT {
        scenario
            .create_and_start(&mut client, &format!("Agent {number}"), STUBBORN_CLI)
            .await;
    }
    let marks = scenario.wait_for_stored_processes(AGENT_COUNT).await;
    kill_server(&mut server);
    drop(client);
    assert_eq!(marks.len(), AGENT_COUNT * 2);
    assert!(
        marks.iter().all(Mark::still_runs),
        "every stand-in outlives the server"
    );

    let restarted = Instant::now();
    let _second = scenario.spawn();
    wait_until_started(&scenario.config, 2).await;
    let mut client = scenario.client().await;
    let plan = plan_after_the_sweep(&mut client, SC003_BOUND).await;
    let measured = restarted.elapsed();
    eprintln!("30 agents, default grace: the plan listed after {measured:?}");

    assert!(measured < SC003_BOUND, "SC-003: {measured:?}");
    assert_all_gone(&marks);
    assert_eq!(
        plan["agents"].as_array().expect("agents").len(),
        AGENT_COUNT
    );
    let entries = scenario.sweep_entries();
    assert_eq!(entries.len(), marks.len(), "one entry per stored identity");
    for mark in &marks {
        let entry = scenario.entry_for(mark.pid);
        assert_eq!(entry.actor, "office");
        assert!(entry.agent.is_some(), "{entry:?}");
        assert_ended_by_the_sweep(&entry);
    }
    assert!(
        forced_count(&entries) >= AGENT_COUNT,
        "every helper ignores the request: {entries:?}"
    );
}

#[tokio::test]
async fn a_cli_and_its_helper_that_end_on_the_request_are_ended_and_no_agent_starts() {
    let mut scenario = Scenario::new();
    let mut server = scenario.spawn();
    wait_until_started(&scenario.config, 1).await;
    let mut client = scenario.client().await;
    let (agent, _) = scenario
        .create_and_start(&mut client, "Ana", PLAIN_CLI)
        .await;
    let marks = scenario.wait_for_stored_processes(1).await;
    kill_server(&mut server);
    drop(client);

    let _second = scenario.spawn();
    wait_until_started(&scenario.config, 2).await;
    let mut client = scenario.client().await;
    let plan = plan_after_the_sweep(&mut client, SC003_BOUND).await;

    assert_all_gone(&marks);
    for mark in &marks {
        let entry = scenario.entry_for(mark.pid);
        assert!(
            entry
                .detail
                .contains(&format!(": ended: pid {}, session ", mark.pid)),
            "{entry:?}"
        );
        assert_eq!(entry.result, "ok");
    }
    assert_eq!(plan["agents"][0]["agent_id"], agent.as_str(), "{plan}");
    assert_eq!(client.status_of(&agent).await, "interrupted");
    let state = client.ask("sweep.state", json!({})).await;
    assert_eq!(state["result"]["state"], "done", "{state}");
}

#[tokio::test]
async fn the_sweep_state_is_asked_and_announced_while_a_stubborn_cli_is_ended_by_force() {
    let mut scenario = Scenario::new();
    let mut server = scenario.spawn();
    wait_until_started(&scenario.config, 1).await;
    let mut client = scenario.client().await;
    set_grace(&mut client, SHORT_GRACE_MS).await;
    scenario
        .create_and_start(&mut client, "Ana", STUBBORN_CLI)
        .await;
    let marks = scenario.wait_for_stored_processes(1).await;
    kill_server(&mut server);
    drop(client);

    let _second = scenario.spawn();
    wait_until_started(&scenario.config, 2).await;
    let mut client = scenario.client().await;
    let running = client.ask("sweep.state", json!({})).await;
    let announced = client.next_event("office.sweep").await;

    assert_eq!(running["result"]["state"], "running", "{running}");
    assert_eq!(announced["state"], "done", "{announced}");
    assert_all_gone(&marks);
    for mark in &marks {
        assert_ended_by_the_sweep(&scenario.entry_for(mark.pid));
    }
    assert!(forced_count(&scenario.sweep_entries()) >= 1);
}

#[tokio::test]
async fn a_process_with_another_start_time_or_command_or_without_an_identity_is_not_ended() {
    let mut scenario = Scenario::new();
    let mut server = scenario.spawn();
    wait_until_started(&scenario.config, 1).await;
    let mut client = scenario.client().await;
    for name in ["Ana", "Raj", "Eli"] {
        scenario
            .create_and_start(&mut client, name, PLAIN_CLI)
            .await;
    }
    scenario.wait_for_stored_processes(3).await;
    // The decoys are helpers: a plain CLI ends by itself when its helper ends.
    let helpers = scenario.pids_in("helper-pids");
    let (other_start_time, other_command, control) = (helpers[0], helpers[1], helpers[2]);
    kill_server(&mut server);
    drop(client);
    // The stored identity of one process now says another start time, and
    // that of another one says another command line: what a process id that
    // the OS gave to a different process would look like to the sweep.
    let database = scenario.database();
    database
        .execute(
            "UPDATE agent_session_processes SET started_at = started_at + 1 WHERE pid = ?1",
            [other_start_time],
        )
        .expect("another start time");
    database
        .execute(
            "UPDATE agent_session_processes SET command_hash = zeroblob(32) WHERE pid = ?1",
            [other_command],
        )
        .expect("another command line");
    drop(database);
    let stranger = Command::new("sleep")
        .arg("602")
        .spawn()
        .expect("a stranger");
    let stranger_mark = Mark::read(stranger.id()).expect("the stranger runs");
    scenario.decoys.push(stranger);

    let _second = scenario.spawn();
    wait_until_started(&scenario.config, 2).await;
    let mut client = scenario.client().await;
    plan_after_the_sweep(&mut client, SC003_BOUND).await;

    let still_running = |pid: u32| {
        scenario
            .marks
            .iter()
            .any(|mark| mark.pid == pid && mark.still_runs())
    };
    assert!(still_running(other_start_time));
    assert!(still_running(other_command));
    assert!(stranger_mark.still_runs());
    assert!(!still_running(control));
    let left_by_time = scenario.entry_for(other_start_time);
    let left_by_command = scenario.entry_for(other_command);
    assert!(
        left_by_time.detail.contains(": left: other start time: "),
        "{left_by_time:?}"
    );
    assert!(
        left_by_command
            .detail
            .contains(": left: other command line: "),
        "{left_by_command:?}"
    );
    assert!(scenario.entry_for(control).detail.contains(": ended: "));
    let mentions_stranger = scenario
        .sweep_entries()
        .iter()
        .any(|entry| entry.detail.contains(&format!("pid {}", stranger_mark.pid)));
    assert!(!mentions_stranger, "the office never stored the stranger");
}

#[tokio::test]
async fn a_start_a_new_task_and_a_continue_sent_during_the_sweep_run_after_it() {
    let mut scenario = Scenario::new();
    let mut server = scenario.spawn();
    wait_until_started(&scenario.config, 1).await;
    let mut client = scenario.client().await;
    set_grace(&mut client, LONG_GRACE_MS).await;
    scenario
        .create_and_start(&mut client, "Stubborn", STUBBORN_CLI)
        .await;
    let to_start = scenario.create(&mut client, "To start", QUICK_CLI).await;
    let for_task = scenario.create(&mut client, "For task", QUICK_CLI).await;
    let (_, to_continue) = scenario
        .create_and_start(&mut client, "To continue", "claude")
        .await;
    let marks = scenario.wait_for_stored_processes(1).await;
    wait_for_records(scenario.records.path(), 1).await;
    kill_server(&mut server);
    drop(client);

    let _second = scenario.spawn();
    wait_until_started(&scenario.config, 2).await;
    let mut client = scenario.client().await;
    let plan = client.ask("resume.plan", json!({})).await;
    assert_eq!(plan["result"], json!({"sweeping": true, "agents": []}));
    let sent = Instant::now();
    let start = client
        .send("agent.start", json!({"agent_id": to_start}))
        .await;
    let new_task = client
        .send("agent.new_task", json!({"agent_id": for_task}))
        .await;
    let continued = client
        .send("session.continue", json!({"session_id": to_continue}))
        .await;

    for request in [start, new_task, continued] {
        let answer = client.answer_of(request).await;
        assert_eq!(answer["ok"], true, "{answer}");
        assert_all_gone(&marks);
    }

    assert!(
        sent.elapsed() >= Duration::from_millis(1500),
        "the requests waited for the grace period of the sweep"
    );
    for mark in &marks {
        assert_ended_by_the_sweep(&scenario.entry_for(mark.pid));
    }
}

#[tokio::test]
async fn a_temp_create_sent_during_the_sweep_starts_no_cli_until_the_sweep_is_over() {
    let mut scenario = Scenario::new();
    let mut server = scenario.spawn();
    wait_until_started(&scenario.config, 1).await;
    let mut client = scenario.client().await;
    set_grace(&mut client, LONG_GRACE_MS).await;
    scenario
        .create_and_start(&mut client, "Stubborn", STUBBORN_CLI)
        .await;
    let marks = scenario.wait_for_stored_processes(1).await;
    kill_server(&mut server);
    drop(client);

    let _second = scenario.spawn();
    wait_until_started(&scenario.config, 2).await;
    let mut client = scenario.client().await;
    let plan = client.ask("resume.plan", json!({})).await;
    assert_eq!(plan["result"], json!({"sweeping": true, "agents": []}));
    let clis_before = scenario.pids_in("cli-pids").len();
    let sent = Instant::now();
    let create = client
        .send(
            "agent.create",
            json!({
                "name": "Temp",
                "command": QUICK_CLI,
                "working_folder": scenario.work_path,
                "temp": true,
                "objective": "check the build",
            }),
        )
        .await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        scenario.pids_in("cli-pids").len(),
        clis_before,
        "no CLI starts while the sweep runs"
    );

    let answer = client.answer_of(create).await;

    assert_eq!(answer["ok"], true, "{answer}");
    assert_all_gone(&marks);
    assert!(
        sent.elapsed() >= Duration::from_millis(1500),
        "the create waited for the grace period of the sweep"
    );
    wait_for("the temp's CLI", || {
        (scenario.pids_in("cli-pids").len() > clis_before).then_some(())
    })
    .await;
}

#[tokio::test]
async fn a_kill_of_the_server_during_the_sweep_leaves_one_result_for_each_identity() {
    let mut scenario = Scenario::new();
    let mut server = scenario.spawn();
    wait_until_started(&scenario.config, 1).await;
    let mut client = scenario.client().await;
    set_grace(&mut client, LONG_GRACE_MS).await;
    scenario
        .create_and_start(&mut client, "Plain", PLAIN_CLI)
        .await;
    scenario
        .create_and_start(&mut client, "Stubborn", STUBBORN_CLI)
        .await;
    let waiting = scenario.create(&mut client, "Waiting", QUICK_CLI).await;
    let marks = scenario.wait_for_stored_processes(2).await;
    kill_server(&mut server);
    drop(client);

    let mut second = scenario.spawn();
    wait_until_started(&scenario.config, 2).await;
    // The plain pair ends on the request and has its entries; the stubborn
    // pair is in the grace period when the second server dies.
    wait_for("the entries of the plain pair", || {
        (scenario.sweep_entries().len() >= 2).then_some(())
    })
    .await;
    kill_server(&mut second);
    let survivors = marks.iter().filter(|mark| mark.still_runs()).count();
    assert_eq!(survivors, 2, "only the stubborn pair still runs");

    let _third = scenario.spawn();
    wait_until_started(&scenario.config, 3).await;
    let mut client = scenario.client().await;
    let started = client
        .send("agent.start", json!({"agent_id": waiting}))
        .await;
    let answer = client.answer_of(started).await;
    assert_eq!(answer["ok"], true, "{answer}");

    assert_all_gone(&marks);
    let identities: i64 = scenario
        .database()
        .query_row(
            "SELECT COUNT(*) FROM agent_session_processes WHERE state = 'ended' \
             AND session_id IN (SELECT id FROM agent_sessions WHERE exit_result = 'lost')",
            [],
            |row| row.get(0),
        )
        .expect("count");
    let entries = scenario.sweep_entries();
    assert_eq!(
        entries.len(),
        usize::try_from(identities).expect("count"),
        "{entries:?}"
    );
    for mark in &marks {
        scenario.entry_for(mark.pid);
    }
    let open: i64 = scenario
        .database()
        .query_row(
            "SELECT COUNT(*) FROM agent_session_processes WHERE state = 'open' \
             AND session_id IN (SELECT id FROM agent_sessions WHERE exit_result = 'lost')",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(open, 0);
}

#[tokio::test]
async fn ten_clean_stops_and_ten_normal_stops_end_nothing_and_start_no_agent() {
    let mut scenario = Scenario::new();
    let mut server = scenario.spawn();
    wait_until_started(&scenario.config, 1).await;
    let mut client = scenario.client().await;
    let agent = scenario.create(&mut client, "Ana", QUICK_CLI).await;
    for cycle in 1..=STOP_CYCLES {
        if cycle > 1 {
            wait_until_started(&scenario.config, cycle).await;
            client = scenario.client().await;
            assert_nothing_swept_and_nothing_started(&scenario, &mut client, &agent).await;
        }
        scenario.start_and_remember(&mut client, &agent).await;
        let stopped = client.ask("agent.stop", json!({"agent_id": agent})).await;
        assert_eq!(stopped["ok"], true, "{stopped}");
        scenario.start_and_remember(&mut client, &agent).await;
        let restart = client.ask("server.restart", json!({})).await;
        assert_eq!(restart["ok"], true, "{restart}");
        wait_for("the server to exit", || server.0.try_wait().expect("poll")).await;
        server = scenario.spawn();
    }
    wait_until_started(&scenario.config, STOP_CYCLES + 1).await;
    let mut client = scenario.client().await;

    assert_nothing_swept_and_nothing_started(&scenario, &mut client, &agent).await;
}

impl Scenario {
    /// Starts the agent and remembers its CLI for the cleanup.
    async fn start_and_remember(&mut self, client: &mut Client, agent: &str) {
        let known = self.pids_in("cli-pids").len();
        client.start(agent).await;
        wait_for("the CLI pid", || {
            (self.pids_in("cli-pids").len() > known).then_some(())
        })
        .await;
        let pid = *self.pids_in("cli-pids").last().expect("a pid");
        self.marks.push(Mark::read(pid).expect("the CLI runs"));
    }
}

async fn assert_nothing_swept_and_nothing_started(
    scenario: &Scenario,
    client: &mut Client,
    agent: &str,
) {
    let plan = client.ask("resume.plan", json!({})).await;
    assert_eq!(plan["result"]["sweeping"], false, "{plan}");
    assert!(scenario.sweep_entries().is_empty());
    assert_eq!(client.status_of(agent).await, "interrupted");
    assert_all_gone(&scenario.marks);
    let open: i64 = scenario
        .database()
        .query_row(
            "SELECT COUNT(*) FROM agent_session_processes WHERE state = 'open'",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(open, 0);
}

impl Scenario {
    /// Waits until the office stored at least one open identity for the
    /// session.
    async fn wait_for_session_identity(&self, session: &str) {
        wait_for("the stored identity of the session", || {
            let stored: i64 = self
                .database()
                .query_row(
                    "SELECT COUNT(*) FROM agent_session_processes \
                     WHERE state = 'open' AND session_id = ?1",
                    [session],
                    |row| row.get(0),
                )
                .ok()?;
            (stored > 0).then_some(())
        })
        .await;
    }

    /// Stores an open helper identity for the session on a process that the
    /// office never saw, which the sweep reads as it would an identity with
    /// no command line: not readable, so it is left running.
    fn store_unreadable_process(&mut self, session: &str) -> Mark {
        let process = Command::new("bash")
            .arg0("")
            .stdin(Stdio::piped())
            .spawn()
            .expect("a process without a command line");
        let mark = Mark::read(process.id()).expect("the process runs");
        self.decoys.push(process);
        self.database()
            .execute(
                "INSERT INTO agent_session_processes \
                 (session_id, pid, started_at, command_hash, command_display, boot, kind, state) \
                 VALUES (?1, ?2, ?3, zeroblob(32), 'unread', 'boot', 'helper', 'open')",
                rusqlite::params![
                    session,
                    mark.pid,
                    mark.start_time.parse::<i64>().expect("start time")
                ],
            )
            .expect("an unreadable identity");
        mark
    }
}

fn plan_entry<'a>(plan: &'a Value, agent: &str) -> &'a Value {
    plan["agents"]
        .as_array()
        .expect("agents")
        .iter()
        .find(|entry| entry["agent_id"] == agent)
        .unwrap_or_else(|| panic!("no entry for {agent}: {plan}"))
}

/// SC-005 on the server side: the plan tells, for each agent of an unclean
/// stop, what the sweep did. A real `could not end` needs a process of
/// another user, and a real helper limit needs 200 helpers, so those two
/// causes are covered below the server (`unclean_stop.tests.rs`, the
/// repository test of `sweep_summary`); the helper limit flag is set in the
/// database here, as the recorder sets it.
#[tokio::test]
async fn the_plan_says_for_each_lost_session_what_the_sweep_found() {
    let mut scenario = Scenario::new();
    let mut server = scenario.spawn();
    wait_until_started(&scenario.config, 1).await;
    let mut client = scenario.client().await;
    let (ended, _) = scenario
        .create_and_start(&mut client, "Ended", PLAIN_CLI)
        .await;
    let (unreadable, unreadable_session) = scenario
        .create_and_start(&mut client, "Unreadable", PLAIN_CLI)
        .await;
    let marks = scenario.wait_for_stored_processes(2).await;
    let mut quick = Vec::new();
    for name in ["Hangs up", "No identity", "Capped"] {
        let (agent, session) = scenario
            .create_and_start(&mut client, name, QUICK_CLI)
            .await;
        scenario.wait_for_session_identity(&session).await;
        quick.push((agent, session));
    }
    let [
        (hung_up, _),
        (no_identity, no_identity_session),
        (capped, capped_session),
    ] = <[_; 3]>::try_from(quick).expect("three quick agents");
    kill_server(&mut server);
    drop(client);
    let unreadable_mark = scenario.store_unreadable_process(&unreadable_session);
    let database = scenario.database();
    database
        .execute(
            "DELETE FROM agent_session_processes WHERE session_id = ?1",
            [&no_identity_session],
        )
        .expect("no identity");
    database
        .execute(
            "UPDATE agent_sessions SET helpers_capped = 1 WHERE id = ?1",
            [&capped_session],
        )
        .expect("helper limit");
    drop(database);

    let _second = scenario.spawn();
    wait_until_started(&scenario.config, 2).await;
    let mut client = scenario.client().await;
    let plan = plan_after_the_sweep(&mut client, SC003_BOUND).await;

    assert_all_gone(&marks);
    assert!(
        unreadable_mark.still_runs(),
        "an unreadable process is left"
    );
    let expectations = [
        (&ended, json!({"ended": 2}), false),
        (&unreadable, json!({"may_run": "not readable"}), true),
        (&hung_up, Value::Null, false),
        (&no_identity, json!({"may_run": "no stored process"}), true),
        (&capped, json!({"may_run": "helper limit"}), true),
    ];
    for (agent, finding, warns) in expectations {
        let entry = plan_entry(&plan, agent);
        assert_eq!(entry["unclean_stop"], finding, "{entry}");
        assert_eq!(entry["after_unclean_stop"], warns, "{entry}");
    }
}
