// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! The server as a process of its own, for a test that stops it the way a
//! real stop does: a restart request or a kill.

use std::fs;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use office_harnessd::config::Config;
use serde_json::Value;

use super::fake_cli::FakeClis;
use super::office_client::Client;
use super::wire_client::WireClient;

const STARTED_LINE: &str = "server started";
const START_LIMIT: Duration = Duration::from_secs(60);
const POLL: Duration = Duration::from_millis(50);

/// A server process that is killed when the test ends.
pub struct ServerProcess(pub Child);

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Starts the server binary with the stand-in login shell of `clis` and no
/// other variable of the test's environment.
pub fn spawn_server(config: &Config, clis: &FakeClis) -> ServerProcess {
    ServerProcess(
        Command::new(env!("CARGO_BIN_EXE_office-harnessd"))
            .arg("--data-folder")
            .arg(&config.data_folder)
            .env_clear()
            .env("SHELL", clis.tool_folder().with_file_name("login-shell"))
            .env("HOME", clis.home_folder())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn server"),
    )
}

/// Waits until the log shows that the server started `starts` times.
pub async fn wait_until_started(config: &Config, starts: usize) {
    let deadline = Instant::now() + START_LIMIT;
    loop {
        let log = fs::read_to_string(config.log_path()).unwrap_or_default();
        let started = log
            .lines()
            .filter(|line| line.contains(STARTED_LINE))
            .count();
        if started >= starts {
            return;
        }
        assert!(Instant::now() < deadline, "server did not start: {log}");
        tokio::time::sleep(POLL).await;
    }
}

/// A client that has authenticated with the server that `server.json` names.
pub async fn connect(config: &Config) -> Client {
    let server_file: Value =
        serde_json::from_str(&fs::read_to_string(config.server_file_path()).expect("server file"))
            .expect("server file json");
    let port = u16::try_from(server_file["port"].as_u64().expect("port")).expect("port range");
    let token = fs::read_to_string(config.token_path()).expect("token");
    Client::new(WireClient::authenticate_ok(port, &token).await)
}
