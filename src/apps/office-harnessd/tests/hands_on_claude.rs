// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]
#![allow(clippy::print_stderr, reason = "the run prints its timeline")]

//! 0024 hands-on run with a real, signed-in `claude` (spec A1, SC-001,
//! SC-003, SC-011). It uses the owner's account and the real login
//! environment, so it is ignored by default:
//!
//! `cargo test -p office-harnessd --test hands_on_claude -- --ignored --nocapture`
//!
//! `a_real_claude_idles_with_no_status_change_and_one_notification` takes
//! `HANDS_ON_IDLE_MINUTES` (30 by default) of wall time.
//!
//! It prints a timeline of the status events instead of asserting timings,
//! because the answer time of the model is not ours; the checks that it does
//! make are the ones that do not depend on it.

mod support;

use std::time::{Duration, Instant};

use office_harnessd::bootstrap::LoginSetup;
use office_harnessd::server::ServerOptions;
use serde_json::{Value, json};
use support::office_client::Client;
use support::running_office::serve_office_with;
use support::wire_client::{Received, WireClient};

const QUIET_PERIOD_MS: u64 = 5_000;
const PROMPT_OK: &str = "cmVwbHkgd2l0aCB0aGUgc2luZ2xlIHdvcmQgb2sN";
const PROMPT_SHELL: &str =
    "Q3JlYXRlIGEgZmlsZSBuYW1lZCBjaGVjay50eHQgY29udGFpbmluZyB0aGUgd29yZCBvaw0=";
const PROMPT_COUNT: &str =
    "V3JpdGUgYW4gZXNzYXkgb2YgYWJvdXQgMjAwMCB3b3JkcyBhYm91dCByaXZlcnMsIG5vIGxpc3RzDQ==";
const PROMPT_COMPACT: &str = "L2NvbXBhY3QN";
const ENTER: &str = "DQ==";
const ARROW_DOWN: &str = "G1tC";
const ESCAPE: &str = "Gw==";

struct Run {
    client: Client,
    agent: String,
    started: Instant,
}

impl Run {
    fn stamp(&self) -> String {
        format!("{:6.2}s", self.started.elapsed().as_secs_f64())
    }

    /// Reads events for `limit`, printing the ones that tell the status, and
    /// returns the last status seen.
    async fn watch(&mut self, label: &str, limit: Duration) -> Vec<(Duration, String)> {
        eprintln!("--- {label}");
        let end = Instant::now() + limit;
        let mut seen = Vec::new();
        while Instant::now() < end {
            // The wire client panics after 10 s of silence, so wait less.
            let wait = end
                .saturating_duration_since(Instant::now())
                .min(Duration::from_secs(4));
            let Ok(Received::Text(text)) =
                tokio::time::timeout(wait, self.client.wire.receive()).await
            else {
                continue;
            };
            let Ok(event) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            let kind = event["type"].as_str().unwrap_or_default();
            let line = match kind {
                "agent.status" if event["agent_id"] == self.agent.as_str() => {
                    format!("status {} {}", event["status"], event["detail"])
                }
                "hook.changed" if event["agent_id"] == self.agent.as_str() => {
                    format!("hook {}", event["hook"])
                }
                "needs_you.changed" => format!(
                    "needs_you {}",
                    event["items"].as_array().map_or(0, Vec::len)
                ),
                "notification.created" => {
                    format!("notification {}", event["notification"]["event"])
                }
                "agent.line" if event["agent_id"] == self.agent.as_str() => {
                    format!("line {}", event["line"])
                }
                _ => continue,
            };
            eprintln!("{}  {line}", self.stamp());
            seen.push((self.started.elapsed(), line));
        }
        seen
    }

    async fn type_in(&mut self, data: &str) {
        let answer = self
            .client
            .ask(
                "terminal.input",
                json!({"agent_id": self.agent, "data": data}),
            )
            .await;
        assert_eq!(answer["ok"], true, "{answer}");
    }

    async fn hook(&mut self) -> Value {
        let listed = self.client.ask("agent.list", json!({})).await;
        listed["result"]
            .as_array()
            .expect("agents")
            .iter()
            .find(|entry| entry["id"] == self.agent.as_str())
            .map(|entry| entry["hook"].clone())
            .expect("listed")
    }
}

/// Prints what the terminals showed, without the escape sequences, for the
/// reader of the timeline.
fn print_output(folder: &std::path::Path) {
    for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
        let bytes = std::fs::read(entry.path()).unwrap_or_default();
        let text = String::from_utf8_lossy(&bytes);
        let mut plain = String::new();
        let mut in_escape = false;
        for character in text.chars() {
            match (in_escape, character) {
                (false, '\u{1b}') => in_escape = true,
                (true, c) if c.is_ascii_alphabetic() || c == '~' => in_escape = false,
                (false, c) if !c.is_control() || c == '\n' => plain.push(c),
                _ => {}
            }
        }
        let tail: String = plain
            .chars()
            .rev()
            .take(1_500)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        eprintln!("--- output {:?}:\n{tail}", entry.file_name());
    }
}

#[tokio::test]
#[ignore = "uses the real claude and the owner's account"]
async fn a_real_claude_reports_its_turns_permission_asks_and_interrupts() {
    let serving = serve_office_with(
        ServerOptions::default(),
        LoginSetup::detect(),
        Some(QUIET_PERIOD_MS),
    )
    .await;
    let folder = tempfile::tempdir().expect("work folder");
    let wire = WireClient::authenticate_ok(serving.office.port, &serving.office.token()).await;
    let mut client = Client::new(wire);
    let created = client
        .ask(
            "agent.create",
            json!({"name": "Real", "command": "claude --model haiku --permission-mode default", "working_folder": folder.path()}),
        )
        .await;
    assert_eq!(created["ok"], true, "{created}");
    let agent = created["result"]["id"].as_str().expect("id").to_owned();
    let mut run = Run {
        client,
        agent: agent.clone(),
        started: Instant::now(),
    };

    // A project hook of the person: it must run once for each turn that the
    // office hooks also see (0024/FR-003).
    let marker = folder.path().join("marker");
    let project_settings = folder.path().join(".claude");
    std::fs::create_dir_all(&project_settings).expect("project settings folder");
    std::fs::write(
        project_settings.join("settings.json"),
        json!({"hooks": {"UserPromptSubmit": [{"hooks": [{
            "type": "command",
            "command": format!("echo turn >> {}", marker.display()),
        }]}]}})
        .to_string(),
    )
    .expect("project settings");
    let started = run.client.start(&agent).await;
    eprintln!("session started: {}", started["id"]);
    run.watch("start up (trust dialog may show)", Duration::from_secs(12))
        .await;
    // The dialog selects "No, exit" first, so move to "Yes" before Enter.
    run.type_in(ARROW_DOWN).await;
    run.type_in(ENTER).await;
    run.watch("after Enter on the trust dialog", Duration::from_secs(10))
        .await;
    eprintln!("hook after start: {}", run.hook().await);
    print_output(&serving.office.config.output_folder_path());

    run.type_in(PROMPT_OK).await;
    run.watch("turn 1: reply with ok", Duration::from_secs(25))
        .await;
    eprintln!("hook after turn 1: {}", run.hook().await);

    run.type_in(PROMPT_SHELL).await;
    run.watch(
        "turn 2: create a file (expect a permission ask)",
        Duration::from_secs(25),
    )
    .await;
    eprintln!("hook during ask: {}", run.hook().await);
    run.type_in(ENTER).await;
    run.watch("after approving", Duration::from_secs(25)).await;
    eprintln!("hook after approval: {}", run.hook().await);

    run.type_in(PROMPT_COUNT).await;
    run.watch(
        "turn 3: long output, then Escape after 3 s",
        Duration::from_secs(3),
    )
    .await;
    run.type_in(ESCAPE).await;
    run.watch(
        "after Escape (no Stop report expected)",
        Duration::from_secs(20),
    )
    .await;
    eprintln!("hook after escape: {}", run.hook().await);

    run.type_in(PROMPT_COMPACT).await;
    run.watch("manual /compact", Duration::from_secs(40)).await;
    eprintln!("hook after compact: {}", run.hook().await);

    eprintln!(
        "project hook markers: {}",
        std::fs::read_to_string(&marker)
            .unwrap_or_default()
            .lines()
            .count()
    );
    let _ = run
        .client
        .ask("agent.stop", json!({"agent_id": agent}))
        .await;
    run.watch("stop", Duration::from_secs(5)).await;
    serving.stop().await;
}

/// Brings a real `claude` to its prompt in a folder that it trusts, as the
/// first test does, and returns the run and the server.
async fn trusted_claude() -> (
    Run,
    support::running_office::ServingOffice,
    tempfile::TempDir,
) {
    let serving = serve_office_with(
        ServerOptions::default(),
        LoginSetup::detect(),
        Some(QUIET_PERIOD_MS),
    )
    .await;
    let folder = tempfile::tempdir().expect("work folder");
    let wire = WireClient::authenticate_ok(serving.office.port, &serving.office.token()).await;
    let mut client = Client::new(wire);
    let created = client
        .ask(
            "agent.create",
            json!({"name": "Real", "command": "claude --model haiku", "working_folder": folder.path()}),
        )
        .await;
    let agent = created["result"]["id"].as_str().expect("id").to_owned();
    let mut run = Run {
        client,
        agent: agent.clone(),
        started: Instant::now(),
    };
    run.client.start(&agent).await;
    run.watch("start up", Duration::from_secs(12)).await;
    run.type_in(ARROW_DOWN).await;
    run.type_in(ENTER).await;
    run.watch("trust accepted", Duration::from_secs(10)).await;
    (run, serving, folder)
}

#[tokio::test]
#[ignore = "uses the real claude, the owner's account, and 30 minutes"]
async fn a_real_claude_idles_with_no_status_change_and_one_notification() {
    let minutes: u64 = std::env::var("HANDS_ON_IDLE_MINUTES")
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(30);
    let (mut run, serving, _folder) = trusted_claude().await;
    run.type_in(PROMPT_OK).await;
    let turn = run.watch("one turn", Duration::from_secs(30)).await;
    let notifications_of_turn = turn
        .iter()
        .filter(|(_, line)| line.starts_with("notification"))
        .count();
    eprintln!("notifications for the turn: {notifications_of_turn}");

    let idle = run
        .watch("idle, no input", Duration::from_secs(minutes * 60))
        .await;
    let changes = idle
        .iter()
        .filter(|(_, line)| line.starts_with("status"))
        .count();
    let notifications = idle
        .iter()
        .filter(|(_, line)| line.starts_with("notification"))
        .count();
    eprintln!("idle {minutes} min: {changes} status changes, {notifications} notifications");
    assert_eq!(changes, 0, "{idle:?}");
    assert_eq!(notifications, 0, "{idle:?}");
    serving.stop().await;
}
