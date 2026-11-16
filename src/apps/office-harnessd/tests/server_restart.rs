// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]
#![cfg(unix)]

//! `server.restart` against the real server process: it answers, ends the
//! running agents, closes the connections with 1001, removes `server.json`,
//! and exits with code 0.

mod support;

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::ExitResult;
use agent_infrastructure::persistence::{AgentRepository, SessionRepository, SharedConnection};
use rusqlite::Connection;
use serde_json::{Value, json};
use support::config_in;
use support::office_client::{Client, wait_for};
use support::wire_client::{Received, WireClient};

const WAIT_LIMIT: Duration = Duration::from_secs(30);
const POLL: Duration = Duration::from_millis(50);
const CLOSE_GOING_AWAY: u16 = 1001;
const SLEEP_PROGRAMS: [&str; 2] = ["/bin/sleep", "/usr/bin/sleep"];

/// A server process that is killed when the test ends.
struct ServerProcess(Child);

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn announced_port(server_file: &Path) -> Option<u16> {
    let text = std::fs::read_to_string(server_file).ok()?;
    let announcement: Value = serde_json::from_str(&text).ok()?;
    u16::try_from(announcement["port"].as_u64()?).ok()
}

#[tokio::test]
async fn a_restart_request_ends_the_agents_closes_the_sockets_and_exits_with_code_zero() {
    let folder = tempfile::tempdir().expect("folder");
    let config = config_in(&folder);
    let work_folder = tempfile::tempdir().expect("work folder");
    let mut server = ServerProcess(
        Command::new(env!("CARGO_BIN_EXE_office-harnessd"))
            .arg("--data-folder")
            .arg(&config.data_folder)
            // A plain shell answers the login-environment query at once.
            .env("SHELL", "/bin/sh")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn server"),
    );
    let server_file = config.server_file_path();
    let port = wait_for("server.json", || announced_port(&server_file)).await;
    let token = std::fs::read_to_string(config.token_path()).expect("token");
    let mut client = Client::new(WireClient::authenticate_ok(port, &token).await);
    let sleeper = SLEEP_PROGRAMS
        .iter()
        .find(|program| Path::new(program).exists())
        .expect("a sleep program");
    let created = client
        .ask(
            "agent.create",
            json!({"name": "Ana", "command": format!("{sleeper} 60"), "working_folder": work_folder.path()}),
        )
        .await;
    let agent = created["result"]["id"].as_str().expect("id").to_owned();
    client.start(&agent).await;
    assert!(
        ["starting", "working", "waiting_for_input"]
            .contains(&client.status_of(&agent).await.as_str())
    );

    let answer = client.ask("server.restart", json!({})).await;

    assert_eq!(answer["ok"], true, "the answer comes first: {answer}");
    let close = loop {
        match client.wire.receive().await {
            Received::Closed(code) => break code,
            Received::Text(_) | Received::Other => {}
        }
    };
    assert_eq!(close, Some(CLOSE_GOING_AWAY));
    let deadline = Instant::now() + WAIT_LIMIT;
    let exit = loop {
        if let Some(status) = server.0.try_wait().expect("poll") {
            break status;
        }
        assert!(Instant::now() < deadline, "the server did not exit");
        std::thread::sleep(POLL);
    };
    assert_eq!(exit.code(), Some(0));
    assert!(!server_file.exists(), "server.json is removed");
    let connection =
        SharedConnection::new(Connection::open(config.database_path()).expect("database"));
    let stored = AgentRepository::new(connection.clone())
        .find(agent.parse().expect("id"))
        .expect("find")
        .expect("agent");
    assert_eq!(
        stored.status,
        AgentStatus::Interrupted,
        "a running agent is no longer running"
    );
    let session = SessionRepository::new(connection)
        .latest_for_agent(agent.parse().expect("id"))
        .expect("latest")
        .expect("session");
    assert_eq!(session.exit_result, Some(ExitResult::Interrupted));
}
