// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! What the secret tests share (0026): agents whose CLI hands its call
//! credential to the test, the real helper run as such an agent, and a scan
//! of every byte the office wrote.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::{Value, json};

use super::office_client::{Client, Office, wait_for};

/// The real server binary that the fake CLIs run as the helper.
pub const HELPER: &str = env!("CARGO_BIN_EXE_office-harnessd");

pub const FIRST_VALUE: &str = "first-marker-value-0026";
pub const SECOND_VALUE: &str = "second-marker-value-0026";

/// A fake CLI body that hands its session's environment to
/// `<records>/<name>.env` and stays alive.
pub fn agent_body(records: &Path, name: &str) -> String {
    let folder = records.display();
    format!("env > {folder}/{name}.part\nmv {folder}/{name}.part {folder}/{name}.env\nsleep 60\n")
}

pub struct Caller {
    pub credential: String,
    pub server_file: String,
}

pub async fn caller_of(records: &Path, name: &str) -> Caller {
    let path = records.join(format!("{name}.env"));
    let env = wait_for("the agent's environment", || fs::read_to_string(&path).ok()).await;
    let value = |variable: &str| {
        env.lines()
            .find_map(|line| line.strip_prefix(&format!("{variable}=")))
            .expect(variable)
            .to_owned()
    };
    Caller {
        credential: value("OFFICE_HARNESS_AGENT_TOKEN"),
        server_file: value("OFFICE_HARNESS_SERVER_FILE"),
    }
}

/// Runs the helper as the agent, on a blocking thread: waiting for it on the
/// runtime thread would stop the server it talks to.
pub async fn helper(caller: &Caller, arguments: Vec<String>) -> Output {
    let credential = caller.credential.clone();
    let server_file = caller.server_file.clone();
    tokio::task::spawn_blocking(move || {
        Command::new(HELPER)
            .arg("agent-call")
            .args(arguments)
            .env("OFFICE_HARNESS_AGENT_TOKEN", credential)
            .env("OFFICE_HARNESS_SERVER_FILE", server_file)
            .output()
            .expect("helper")
    })
    .await
    .expect("helper task")
}

pub async fn call(caller: &Caller, kind: &str, fields: &Value) -> Value {
    let outcome = helper(caller, vec![kind.to_owned(), fields.to_string()]).await;
    serde_json::from_str(String::from_utf8_lossy(&outcome.stdout).trim()).expect("json")
}

pub async fn running_agent(office: &Office, client: &mut Client, name: &str) -> (String, Caller) {
    office
        .clis
        .add(name, &agent_body(office.records_path(), name));
    let agent_id = client.create_agent(office, name, name, None).await;
    client.start(&agent_id).await;
    let caller = caller_of(office.records_path(), name).await;
    (agent_id, caller)
}

pub async fn save(client: &mut Client, name: &str, value: &str) -> Value {
    client
        .ask("secret.set", json!({"name": name, "value": value}))
        .await
}

pub async fn grant(client: &mut Client, name: &str, agent_id: &str) -> Value {
    client
        .ask("secret.grant", json!({"name": name, "agent_id": agent_id}))
        .await
}

pub async fn actions_of(client: &mut Client, agent_id: &str) -> Vec<Value> {
    let actions = client
        .ask("agent.actions", json!({"agent_id": agent_id}))
        .await;
    assert_eq!(actions["ok"], true, "{actions}");
    actions["result"].as_array().expect("entries").clone()
}

pub fn kind_entries<'entries>(entries: &'entries [Value], kind: &str) -> Vec<&'entries Value> {
    entries
        .iter()
        .filter(|entry| entry["action"] == kind)
        .collect()
}

/// Every byte the office wrote to its data folder.
pub fn data_folder_bytes(folder: &Path) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut folders = vec![folder.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder).expect("folder").flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else if let Ok(content) = fs::read(&path) {
                bytes.extend(content);
            }
        }
    }
    bytes
}

pub fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}
