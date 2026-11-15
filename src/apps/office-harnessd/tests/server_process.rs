// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

mod support;

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use agent_domain::enums::AgentStatus;
use agent_infrastructure::persistence::{AgentRepository, MachineRepository, SharedConnection};
use office_harnessd::bootstrap::ALREADY_RUNNING_EXIT_CODE;
use office_harnessd::config::Config;
use rusqlite::Connection;
use support::{agent_named, config_in};

const STARTED_LINE: &str = "server started";
const ALREADY_RUNNING_LINE: &str = "another server already runs";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// A running server process that is killed when the test ends.
struct RunningServer(Child);

impl Drop for RunningServer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn_server(config: &Config) -> Child {
    Command::new(env!("CARGO_BIN_EXE_office-harnessd"))
        .arg("--data-folder")
        .arg(&config.data_folder)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn server")
}

fn log_text(config: &Config) -> String {
    std::fs::read_to_string(config.log_path()).unwrap_or_default()
}

fn count_lines_containing(text: &str, needle: &str) -> usize {
    text.lines().filter(|line| line.contains(needle)).count()
}

fn wait_for_started_lines(config: &Config, expected: usize) {
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    while count_lines_containing(&log_text(config), STARTED_LINE) < expected {
        assert!(
            Instant::now() < deadline,
            "server did not start: {}",
            log_text(config)
        );
        sleep(POLL_INTERVAL);
    }
}

fn open_agents(database: &Path) -> (MachineRepository, AgentRepository) {
    let connection = SharedConnection::new(Connection::open(database).expect("open database"));
    (
        MachineRepository::new(connection.clone()),
        AgentRepository::new(connection),
    )
}

#[test]
fn a_second_server_exits_with_a_log_line_and_the_first_keeps_running() {
    let folder = tempfile::tempdir().expect("folder");
    let config = config_in(&folder);
    let mut first = RunningServer(spawn_server(&config));
    wait_for_started_lines(&config, 1);

    let second = spawn_server(&config)
        .wait_with_output()
        .expect("second server");

    assert_eq!(
        second.status.code(),
        Some(i32::from(ALREADY_RUNNING_EXIT_CODE))
    );
    assert!(String::from_utf8_lossy(&second.stderr).contains(ALREADY_RUNNING_LINE));
    assert!(log_text(&config).contains(ALREADY_RUNNING_LINE));
    assert!(
        first.0.try_wait().expect("poll first").is_none(),
        "the first server stopped"
    );
}

#[test]
fn after_an_unclean_stop_running_agents_become_interrupted_and_none_starts() {
    let folder = tempfile::tempdir().expect("folder");
    let config = config_in(&folder);
    let mut first = RunningServer(spawn_server(&config));
    wait_for_started_lines(&config, 1);
    let (machines, agents) = open_agents(&config.database_path());
    let agent = agent_named(machines.ensure_local("ignored").expect("machine").id, "Ana");
    agents.insert(&agent).expect("agent");
    agents
        .update_status(agent.id, AgentStatus::Working)
        .expect("working");

    first
        .0
        .kill()
        .expect("kill the first server without a clean stop");
    first.0.wait().expect("reap");
    let _second = RunningServer(spawn_server(&config));
    wait_for_started_lines(&config, 2);

    let status = agents.find(agent.id).expect("find").expect("exists").status;
    assert_eq!(status, AgentStatus::Interrupted);
}
