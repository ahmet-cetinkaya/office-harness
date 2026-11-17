// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! An agent calls the office from inside its session (0002, T4). The fake CLI
//! runs the real `office-harnessd agent-call` helper with the environment
//! that the server gave the session.

mod support;

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::{Value, json};
use support::office_client::{Office, wait_for};

/// The real server binary that the fake CLI runs as the helper.
const HELPER: &str = env!("CARGO_BIN_EXE_office-harnessd");

/// A fake CLI that records what the helper said for one call, then waits.
/// `calls` is a list of `<name> <request type>` pairs; the helper's answer
/// goes to `<records>/<name>.out`, and its environment to `<records>/env`.
fn call_script(records: &Path, calls: &[(&str, &str)]) -> String {
    let folder = records.display();
    let mut body = format!("env > {folder}/env.part\nmv {folder}/env.part {folder}/env\n");
    for (name, kind) in calls {
        let _ = writeln!(
            body,
            "{HELPER} agent-call {kind} > {folder}/{name}.out 2> {folder}/{name}.err\nprintf '%s' $? > {folder}/{name}.code"
        );
    }
    body.push_str("sleep 60\n");
    body
}

async fn read_output(records: &Path, name: &str) -> (String, String) {
    let code = wait_for("the helper's exit code", || {
        fs::read_to_string(records.join(format!("{name}.code"))).ok()
    })
    .await;
    let out = fs::read_to_string(records.join(format!("{name}.out"))).expect("out");
    (code, out)
}

fn environment_value(records: &Path, name: &str) -> String {
    let env = fs::read_to_string(records.join("env")).expect("env");
    env.lines()
        .find_map(|line| line.strip_prefix(&format!("{name}=")))
        .unwrap_or_default()
        .to_owned()
}

#[tokio::test]
async fn an_agent_proves_itself_and_the_office_names_it() {
    let office = Office::open().await;
    office.clis.add(
        "claude",
        &call_script(office.records_path(), &[("whoami", "agent.whoami")]),
    );
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "claude", None).await;
    client.start(&agent).await;

    let (code, out) = read_output(office.records_path(), "whoami").await;

    assert_eq!(code, "0", "{out}");
    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    assert_eq!(answer["ok"], true);
    assert_eq!(answer["result"]["agent_id"], agent);
}

#[tokio::test]
async fn an_agent_may_not_ask_for_a_type_outside_the_allowlist() {
    let office = Office::open().await;
    let kinds = [
        ("terminal", "terminal.input"),
        ("delete", "agent.delete"),
        ("team", "team.create"),
        ("tree", "hierarchy.tree"),
        ("restart", "server.restart"),
        ("settings", "settings.set"),
    ];
    office
        .clis
        .add("claude", &call_script(office.records_path(), &kinds));
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "claude", None).await;
    client.start(&agent).await;

    for (name, kind) in kinds {
        let (code, out) = read_output(office.records_path(), name).await;
        let answer: Value = serde_json::from_str(out.trim()).expect("json");
        assert_eq!(code, "1", "{kind}: {out}");
        assert_eq!(answer["ok"], false, "{kind}");
        assert_eq!(answer["error"]["code"], "refused", "{kind}");
    }
}

#[tokio::test]
async fn the_child_gets_its_own_credential_and_never_the_office_token() {
    let office = Office::open().await;
    office.clis.add(
        "claude",
        &call_script(office.records_path(), &[("whoami", "agent.whoami")]),
    );
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "claude", None).await;
    client.start(&agent).await;
    read_output(office.records_path(), "whoami").await;

    let credential = environment_value(office.records_path(), "OFFICE_HARNESS_AGENT_TOKEN");
    let office_token = office.serving.office.token();
    let env_text = fs::read_to_string(office.records_path().join("env")).expect("env");

    assert!(credential.starts_with(&format!("{agent}:")), "{credential}");
    assert!(!env_text.contains(&office_token), "the office token leaked");
    for name in ["OFFICE_HARNESS_CLI", "OFFICE_HARNESS_SERVER_FILE"] {
        assert_ne!(environment_value(office.records_path(), name), "", "{name}");
    }
}

/// Runs the helper with the given credential. The server of this test runs
/// in this test's runtime, so the helper runs on a blocking thread: waiting
/// for it on the runtime thread would stop the server it talks to.
async fn call_with(credential: &str, server_file: &str) -> Output {
    let credential = credential.to_owned();
    let server_file = server_file.to_owned();
    tokio::task::spawn_blocking(move || {
        Command::new(HELPER)
            .args(["agent-call", "agent.whoami"])
            .env("OFFICE_HARNESS_AGENT_TOKEN", credential)
            .env("OFFICE_HARNESS_SERVER_FILE", server_file)
            .output()
            .expect("helper")
    })
    .await
    .expect("helper task")
}

#[tokio::test]
async fn a_stopped_agents_credential_stops_working_and_a_restart_gets_a_new_one() {
    let office = Office::open().await;
    office.clis.add(
        "claude",
        &call_script(office.records_path(), &[("whoami", "agent.whoami")]),
    );
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "claude", None).await;
    client.start(&agent).await;
    read_output(office.records_path(), "whoami").await;
    let first = environment_value(office.records_path(), "OFFICE_HARNESS_AGENT_TOKEN");
    let server_file = environment_value(office.records_path(), "OFFICE_HARNESS_SERVER_FILE");
    // The credential works while the session runs.
    let live = call_with(&first, &server_file).await;
    assert!(live.status.success(), "{live:?}");

    let stopped = client.ask("agent.stop", json!({"agent_id": agent})).await;
    assert_eq!(stopped["ok"], true, "{stopped}");
    client.wait_for_status(&agent, "stopped").await;

    let refused = call_with(&first, &server_file).await;
    assert!(!refused.status.success());
    let secret = first.split_once(':').expect("shape").1;
    let shown = format!(
        "{}{}",
        String::from_utf8_lossy(&refused.stdout),
        String::from_utf8_lossy(&refused.stderr)
    );
    assert!(!shown.contains(secret), "the secret was printed");

    // A new session of the same agent gets a different credential, and the
    // old one stays dead.
    fs::remove_file(office.records_path().join("env")).expect("old env");
    client.start(&agent).await;
    wait_for("the new session's environment", || {
        fs::read_to_string(office.records_path().join("env")).ok()
    })
    .await;
    let second = environment_value(office.records_path(), "OFFICE_HARNESS_AGENT_TOKEN");
    assert_ne!(first, second);
    assert!(!call_with(&first, &server_file).await.status.success());
    assert!(call_with(&second, &server_file).await.status.success());
}
