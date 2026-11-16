// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! A server with fake CLIs, and a client that numbers its own requests.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use office_harnessd::server::ServerOptions;
use serde_json::{Value, json};
use tempfile::TempDir;

use super::fake_cli::FakeClis;
use super::running_office::{ServingOffice, serve_office_with};
use super::wire_client::WireClient;

const QUIET_PERIOD_MS: u64 = 400;
const WAIT_LIMIT: Duration = Duration::from_secs(20);
const POLL: Duration = Duration::from_millis(25);
/// A batch file's `echo` ends the value with a line ending; a shell's `printf`
/// does not.
const LINE_ENDINGS: [char; 2] = ['\r', '\n'];

pub struct Office {
    pub serving: ServingOffice,
    pub clis: FakeClis,
    pub work_folder: TempDir,
    /// Where the fake CLIs write what they received.
    pub records: TempDir,
}

impl Office {
    pub async fn open() -> Self {
        Self::open_with(FakeClis::new()).await
    }

    pub async fn open_with(clis: FakeClis) -> Self {
        let serving = serve_office_with(
            ServerOptions::default(),
            clis.login_setup(),
            Some(QUIET_PERIOD_MS),
        )
        .await;
        Self {
            serving,
            clis,
            work_folder: tempfile::tempdir().expect("work folder"),
            records: tempfile::tempdir().expect("records"),
        }
    }

    pub async fn client(&self) -> Client {
        let wire =
            WireClient::authenticate_ok(self.serving.office.port, &self.serving.office.token())
                .await;
        Client::new(wire)
    }

    pub fn work_path(&self) -> &str {
        self.work_folder.path().to_str().expect("utf-8 path")
    }

    pub fn records_path(&self) -> &Path {
        self.records.path()
    }
}

pub struct Client {
    pub wire: WireClient,
    next_id: u32,
}

impl Client {
    #[must_use]
    pub fn new(wire: WireClient) -> Self {
        Self { wire, next_id: 1 }
    }

    /// Sends a request and returns its answer, skipping events that arrive
    /// first.
    pub async fn ask(&mut self, kind: &str, fields: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let mut body = fields;
        body["id"] = json!(id);
        body["type"] = json!(kind);
        self.wire.send_text(&body.to_string()).await;
        self.answer_of(id).await
    }

    /// Sends a request without waiting for its answer.
    pub async fn send(&mut self, kind: &str, fields: Value) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let mut body = fields;
        body["id"] = json!(id);
        body["type"] = json!(kind);
        self.wire.send_text(&body.to_string()).await;
        id
    }

    pub async fn answer_of(&mut self, id: u32) -> Value {
        loop {
            let message = self.wire.receive_json().await;
            if message["id"] == json!(id) {
                return message;
            }
        }
    }

    /// Reads events until one of the type `kind` arrives.
    pub async fn next_event(&mut self, kind: &str) -> Value {
        loop {
            let message = self.wire.receive_json().await;
            if message["type"] == kind {
                return message;
            }
        }
    }

    /// Creates an agent in the office's work folder and returns its id.
    pub async fn create_agent(
        &mut self,
        office: &Office,
        name: &str,
        command: &str,
        additions: Option<Value>,
    ) -> String {
        let mut fields =
            json!({"name": name, "command": command, "working_folder": office.work_path()});
        if let Some(additions) = additions {
            fields["additions"] = additions;
        }
        let answer = self.ask("agent.create", fields).await;
        assert_eq!(answer["ok"], true, "{answer}");
        answer["result"]["id"].as_str().expect("id").to_owned()
    }

    pub async fn start(&mut self, agent_id: &str) -> Value {
        let answer = self.ask("agent.start", json!({"agent_id": agent_id})).await;
        assert_eq!(answer["ok"], true, "{answer}");
        answer["result"].clone()
    }

    pub async fn sessions(&mut self, agent_id: &str) -> Vec<Value> {
        let answer = self
            .ask("session.list", json!({"agent_id": agent_id}))
            .await;
        assert_eq!(answer["ok"], true, "{answer}");
        answer["result"].as_array().expect("sessions").clone()
    }

    pub async fn status_of(&mut self, agent_id: &str) -> String {
        let listed = self.ask("agent.list", json!({})).await;
        listed["result"]
            .as_array()
            .expect("agents")
            .iter()
            .find(|agent| agent["id"] == agent_id)
            .and_then(|agent| agent["status"].as_str())
            .expect("the agent is listed")
            .to_owned()
    }

    pub async fn wait_for_status(&mut self, agent_id: &str, wanted: &str) {
        let deadline = Instant::now() + WAIT_LIMIT;
        while self.status_of(agent_id).await != wanted {
            assert!(Instant::now() < deadline, "the agent never became {wanted}");
            tokio::time::sleep(POLL).await;
        }
    }
}

/// The additions that the first session of a test starts with.
pub fn first_additions() -> Value {
    json!({
        "flags": ["--first"],
        "env": [{"name": "OFFICE_KEY", "value": "one"}],
        "startup_prompt": "prompt one",
    })
}

/// The additions that an edit gives, for the next session.
pub fn second_additions() -> Value {
    json!({
        "flags": ["--second"],
        "env": [{"name": "OFFICE_KEY", "value": "two"}],
        "startup_prompt": "prompt two",
    })
}

pub fn error_text(answer: &Value) -> (&str, &str) {
    (
        answer["error"]["code"].as_str().expect("code"),
        answer["error"]["message"].as_str().expect("message"),
    )
}

/// Waits until `check` gives a value.
pub async fn wait_for<T>(what: &str, mut check: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + WAIT_LIMIT;
    loop {
        if let Some(found) = check() {
            return found;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        tokio::time::sleep(POLL).await;
    }
}

/// What one fake CLI start wrote: its arguments, one in each line, and the
/// value of `OFFICE_KEY`.
#[derive(Debug, PartialEq, Eq)]
pub struct Record {
    pub arguments: Vec<String>,
    pub key: String,
}

/// The fake script body that writes a `Record` and then runs `rest`. The
/// arguments file comes last, so its existence means the record is whole.
#[cfg(unix)]
pub fn recording_body(records: &Path, rest: &str) -> String {
    let folder = records.display();
    format!(
        "printf '%s' \"$OFFICE_KEY\" > {folder}/key-$$\nprintf '%s\\n' \"$@\" > {folder}/args-$$\n{rest}"
    )
}

/// As on Unix, for a batch file. A batch file has no process id, so a random
/// number names the record. It writes each argument on its own line to a
/// temporary file and moves that file into place, so the arguments file
/// appears whole. A redirection comes first in each command, so a digit that
/// ends a value is never read as a handle number.
#[cfg(windows)]
pub fn recording_body(records: &Path, rest: &str) -> String {
    let folder = records.display();
    format!(
        "set RECORD=%RANDOM%%RANDOM%\n>\"{folder}\\key-%RECORD%\" echo %OFFICE_KEY%\nfor %%a in (%*) do >>\"{folder}\\partial-%RECORD%\" echo %%a\nmove /y \"{folder}\\partial-%RECORD%\" \"{folder}\\args-%RECORD%\" >nul\n{rest}"
    )
}

/// The records so far, oldest first.
pub fn records_of(records: &Path) -> Vec<Record> {
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(records)
        .expect("records folder")
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("args-"))
        .map(|entry| {
            (
                entry
                    .metadata()
                    .and_then(|meta| meta.modified())
                    .expect("mtime"),
                entry.path(),
            )
        })
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|(_, path)| {
            let pid = path
                .file_name()
                .expect("name")
                .to_string_lossy()
                .replace("args-", "");
            Record {
                arguments: std::fs::read_to_string(&path)
                    .expect("arguments")
                    .lines()
                    .map(str::to_owned)
                    .collect(),
                key: std::fs::read_to_string(records.join(format!("key-{pid}")))
                    .expect("key")
                    .trim_end_matches(LINE_ENDINGS)
                    .to_owned(),
            }
        })
        .collect()
}

pub async fn wait_for_records(records: &Path, count: usize) -> Vec<Record> {
    wait_for("the fake CLI records", || {
        let found = records_of(records);
        (found.len() >= count).then_some(found)
    })
    .await
}
