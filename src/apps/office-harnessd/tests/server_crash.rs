// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]
#![cfg(target_os = "linux")]

//! Crash test (0013 research R1): the server process is killed with SIGKILL
//! while agents run. The test shows which CLI processes outlive the server
//! and that the next server start can continue a session. Since 0017 the
//! sweep at that start ends the orphan before the plan lists the agents. It
//! reads process start times from `/proc`, so it runs on Linux only.

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use office_harnessd::config::Config;
use serde_json::{Value, json};
use support::config_in;
use support::fake_cli::FakeClis;
use support::office_client::{Client, recording_body, wait_for, wait_for_records};
use support::wire_client::WireClient;

const STARTED_LINE: &str = "server started";
const WAIT_LIMIT: Duration = Duration::from_secs(60);
const POLL: Duration = Duration::from_millis(50);
/// Long enough for the kernel to deliver the hang-up and for a CLI that ends
/// on it to be gone.
const HANGUP_SETTLE: Duration = Duration::from_secs(3);
/// The `starttime` field of `/proc/<pid>/stat`, counted from the field after
/// the command name.
const START_TIME_FIELD_AFTER_NAME: usize = 19;
const STATE_FIELD_AFTER_NAME: usize = 0;
const ZOMBIE_STATE: &str = "Z";

/// One process as the test saw it while the first server ran.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ProcessIdentity {
    pid: u32,
    start_time: String,
}

impl ProcessIdentity {
    fn read(pid: u32) -> Option<Self> {
        let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let after_name = stat.rsplit_once(')')?.1;
        let fields: Vec<&str> = after_name.split_whitespace().collect();
        if fields.get(STATE_FIELD_AFTER_NAME) == Some(&ZOMBIE_STATE) {
            return None;
        }
        Some(Self {
            pid,
            start_time: (*fields.get(START_TIME_FIELD_AFTER_NAME)?).to_owned(),
        })
    }

    /// True when the same process still runs: same pid and same start time,
    /// so a reused pid does not count.
    fn still_runs(&self) -> bool {
        Self::read(self.pid).as_ref() == Some(self)
    }

    /// Ends the process and its group (the CLI leads a session of its own),
    /// only while it is still the process the test saw.
    fn end_group(&self) {
        if self.still_runs() {
            let _ = Command::new("kill")
                .args(["-KILL", "--", &format!("-{}", self.pid)])
                .status();
        }
    }
}

/// The server process and every CLI the test saw; all of them end when the
/// test ends, also on a failed assertion.
struct Cleanup {
    server: Option<Child>,
    processes: Vec<ProcessIdentity>,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        if let Some(server) = self.server.as_mut() {
            let _ = server.kill();
            let _ = server.wait();
        }
        for process in &self.processes {
            process.end_group();
        }
    }
}

/// Starts the server binary with the stand-in login shell of `clis` and no
/// other variable of the test's environment.
fn spawn_server(config: &Config, clis: &FakeClis) -> Child {
    Command::new(env!("CARGO_BIN_EXE_office-harnessd"))
        .arg("--data-folder")
        .arg(&config.data_folder)
        .env_clear()
        .env("SHELL", clis.tool_folder().with_file_name("login-shell"))
        .env("HOME", clis.home_folder())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn server")
}

async fn wait_until_started(config: &Config, starts: usize) {
    let deadline = Instant::now() + WAIT_LIMIT;
    loop {
        let log = fs::read_to_string(config.log_path()).unwrap_or_default();
        if log
            .lines()
            .filter(|line| line.contains(STARTED_LINE))
            .count()
            >= starts
        {
            return;
        }
        assert!(Instant::now() < deadline, "server did not start: {log}");
        tokio::time::sleep(POLL).await;
    }
}

async fn connect(config: &Config) -> Client {
    let server_file: Value =
        serde_json::from_str(&fs::read_to_string(config.server_file_path()).expect("server file"))
            .expect("server file json");
    let port = u16::try_from(server_file["port"].as_u64().expect("port")).expect("port range");
    let token = fs::read_to_string(config.token_path()).expect("token");
    Client::new(WireClient::authenticate_ok(port, &token).await)
}

/// The answer of `resume.plan` once the sweep is over; while it runs, the
/// plan holds no agent.
async fn plan_after_the_sweep(client: &mut Client) -> Value {
    let deadline = Instant::now() + WAIT_LIMIT;
    loop {
        let plan = client.ask("resume.plan", json!({})).await;
        if plan["result"]["sweeping"] == false {
            return plan["result"].clone();
        }
        assert!(Instant::now() < deadline, "the sweep did not end: {plan}");
        tokio::time::sleep(POLL).await;
    }
}

async fn create_and_start(client: &mut Client, name: &str, command: &str, folder: &Path) -> Value {
    let created = client
        .ask(
            "agent.create",
            json!({"name": name, "command": command, "working_folder": folder}),
        )
        .await;
    assert_eq!(created["ok"], true, "{created}");
    let agent = created["result"]["id"].as_str().expect("id").to_owned();
    let session = client.start(&agent).await;
    json!({"agent": agent, "session": session})
}

/// A script body that writes its pid to `pid-<name>` and then runs `rest`.
fn pid_then(records: &Path, name: &str, rest: &str) -> String {
    format!(
        "printf '%s' \"$$\" > {}/pid-{name}\n{rest}",
        records.display()
    )
}

async fn pid_of(records: &Path, name: &str) -> ProcessIdentity {
    let path: PathBuf = records.join(format!("pid-{name}"));
    let pid = wait_for(&format!("the pid of {name}"), || {
        fs::read_to_string(&path).ok()?.trim().parse::<u32>().ok()
    })
    .await;
    ProcessIdentity::read(pid).expect("the CLI runs")
}

#[tokio::test]
async fn after_a_sigkill_only_a_cli_that_ignores_the_hangup_outlives_the_server_and_a_session_continues()
 {
    let folder = tempfile::tempdir().expect("folder");
    let work = tempfile::tempdir().expect("work folder");
    let records = tempfile::tempdir().expect("records");
    let config = config_in(&folder);
    let clis = FakeClis::new();
    let _ = clis.login_setup();
    clis.add(
        "ignores-hangup",
        &pid_then(
            records.path(),
            "ignores",
            "trap '' HUP\necho ready\nwhile :; do sleep 1; done",
        ),
    );
    clis.add(
        "ends-on-hangup",
        &pid_then(records.path(), "ends", "echo ready\nexec sleep 600"),
    );
    clis.add(
        "claude",
        &recording_body(
            records.path(),
            &pid_then(records.path(), "claude", "echo ready\nexec sleep 600"),
        ),
    );
    let mut cleanup = Cleanup {
        server: Some(spawn_server(&config, &clis)),
        processes: Vec::new(),
    };
    wait_until_started(&config, 1).await;
    let mut client = connect(&config).await;
    let ignores = create_and_start(&mut client, "Ignores", "ignores-hangup", work.path()).await;
    let ends = create_and_start(&mut client, "Ends", "ends-on-hangup", work.path()).await;
    let claude = create_and_start(&mut client, "Claude", "claude", work.path()).await;
    let ignoring = pid_of(records.path(), "ignores").await;
    let ending = pid_of(records.path(), "ends").await;
    let first_claude = pid_of(records.path(), "claude").await;
    cleanup.processes = vec![ignoring.clone(), ending.clone(), first_claude.clone()];

    let mut server = cleanup.server.take().expect("server");
    server.kill().expect("SIGKILL the server");
    server.wait().expect("reap the server");
    drop(client);
    tokio::time::sleep(HANGUP_SETTLE).await;

    assert!(
        ignoring.still_runs(),
        "a CLI that ignores the hang-up outlives the server"
    );
    assert!(
        !ending.still_runs(),
        "a CLI that ends on the hang-up is gone"
    );
    assert!(!first_claude.still_runs(), "the claude stand-in is gone");

    cleanup.server = Some(spawn_server(&config, &clis));
    wait_until_started(&config, 2).await;
    let mut client = connect(&config).await;
    for started in [&ignores, &ends, &claude] {
        let agent = started["agent"].as_str().expect("agent");
        assert_eq!(client.status_of(agent).await, "interrupted");
    }
    let plan = plan_after_the_sweep(&mut client).await;
    assert!(
        !ignoring.still_runs(),
        "the sweep of 0017 ends the orphan before the plan lists the agents"
    );
    let finding_of = |started: &Value| {
        let agent = started["agent"].as_str().expect("agent");
        plan["agents"]
            .as_array()
            .expect("planned agents")
            .iter()
            .find(|entry| entry["agent_id"] == agent)
            .unwrap_or_else(|| panic!("no entry for {agent}: {plan}"))
            .clone()
    };
    assert_eq!(finding_of(&ignores)["unclean_stop"], json!({"ended": 1}));
    assert_eq!(finding_of(&ends)["unclean_stop"], Value::Null);
    assert_eq!(finding_of(&claude)["unclean_stop"], Value::Null);
    assert_eq!(
        finding_of(&ignores)["after_unclean_stop"],
        false,
        "an ended leftover is not a warning: {plan}"
    );

    let continued = client
        .ask(
            "session.continue",
            json!({"session_id": claude["session"]["id"]}),
        )
        .await;

    assert_eq!(continued["ok"], true, "{continued}");
    let cli_id = claude["session"]["cli_session_id"]
        .as_str()
        .expect("cli id");
    let started = wait_for_records(records.path(), 2).await;
    // The continued claude ends with the server's hang-up at cleanup.
    assert_eq!(
        support::without_hook_settings(&started[1].arguments),
        ["--resume", cli_id]
    );
}
