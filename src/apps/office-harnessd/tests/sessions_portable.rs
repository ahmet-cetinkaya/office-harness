// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! The T7 tests that give SC-005 and SC-009 their evidence on every OS, with
//! fake CLIs started through the whole server and a real socket. The rest of
//! T7 is in `sessions.rs`, which needs a Unix shell.

mod support;

use std::collections::BTreeMap;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde_json::json;
use support::fake_cli::{FakeClis, SHORT_ATTRIBUTION, print_then_wait};
use support::office_client::{
    Client, Office, first_additions, recording_body, records_of, second_additions, wait_for_records,
};

const FRAME_HEADER_BYTES: usize = 17;
/// A stop sends the interrupt key first. On Windows a batch file answers it
/// with "Terminate batch job (Y/N)?" and waits, so the stop ends the child
/// only after the grace period; a short one keeps each request under the
/// client's wait.
const SHORT_STOP_GRACE_MS: &str = "500";
const TERMINAL_TEXT_WAIT: Duration = Duration::from_secs(10);

async fn shorten_the_stop_grace(client: &mut Client) {
    let set = client
        .ask(
            "settings.set",
            json!({"key": "stop_grace_ms", "value": SHORT_STOP_GRACE_MS}),
        )
        .await;
    assert_eq!(set["ok"], true, "{set}");
}

/// Reads what the terminal of `agent` shows, from the snapshot on, until it
/// holds `wanted`. The child cannot record typed input on every OS (a batch
/// file cannot), so the typed startup prompt is proved by its echo.
async fn wait_for_terminal_text(client: &mut Client, agent: &str, wanted: &str) {
    let attached = client
        .ask(
            "terminal.attach",
            json!({"agent_id": agent, "cols": 80, "rows": 24}),
        )
        .await;
    assert_eq!(attached["ok"], true, "{attached}");
    let mut shown = String::new();
    let read_until_wanted = async {
        while !shown.contains(wanted) {
            let frame = client.wire.receive_binary().await;
            shown.push_str(&String::from_utf8_lossy(&frame[FRAME_HEADER_BYTES..]));
        }
    };
    let found = tokio::time::timeout(TERMINAL_TEXT_WAIT, read_until_wanted).await;
    assert!(
        found.is_ok(),
        "the terminal never showed {wanted:?}; it showed {shown:?}"
    );
    client
        .ask("terminal.detach", json!({"agent_id": agent}))
        .await;
}

// (a), (c): additions reach the child at the next session only.

#[tokio::test]
async fn flags_env_and_the_startup_prompt_reach_the_child_at_the_next_session_and_an_edit_leaves_the_running_one()
 {
    let office = Office::open().await;
    office.clis.add(
        "claude",
        &recording_body(office.records_path(), &print_then_wait("ready")),
    );
    let mut client = office.client().await;
    shorten_the_stop_grace(&mut client).await;
    let mut observer = office.client().await;
    let agent = client
        .create_agent(&office, "Ana", "claude", Some(first_additions()))
        .await;
    let first = client.start(&agent).await;
    let records = wait_for_records(office.records_path(), 1).await;
    let cli_id = first["cli_session_id"].as_str().expect("cli id");
    assert_eq!(
        records[0].arguments,
        ["--first", "--session-id", cli_id],
        "the prompt is typed in, never an argument"
    );
    assert_eq!(records[0].key, "one");
    wait_for_terminal_text(&mut client, &agent, "prompt one").await;

    let edited = client
        .ask(
            "agent.additions.set",
            json!({"agent_id": agent, "additions": second_additions()}),
        )
        .await;

    assert_eq!(edited["ok"], true, "{edited}");
    assert_eq!(edited["result"]["applies_next_session"], true);
    let event = observer.next_event("agent.additions.changed").await;
    assert_eq!(event["agent_id"], agent.as_str());
    assert_eq!(event["additions"]["flags"][0], "--second");
    let sessions = client.sessions(&agent).await;
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0]["additions_snapshot"]["flags"][0], "--first");
    assert_eq!(
        records_of(office.records_path()).len(),
        1,
        "the running child is untouched"
    );

    let next = client
        .ask("agent.new_task", json!({"agent_id": agent}))
        .await;
    assert_eq!(next["ok"], true, "{next}");
    let records = wait_for_records(office.records_path(), 2).await;
    let next_id = next["result"]["cli_session_id"].as_str().expect("cli id");
    assert_eq!(
        records[1].arguments,
        ["--second", "--session-id", next_id],
        "the prompt is typed in, never an argument"
    );
    assert_eq!(records[1].key, "two");
    assert_ne!(next_id, cli_id, "a new task is a new session");
    wait_for_terminal_text(&mut client, &agent, "prompt two").await;
    let sessions = client.sessions(&agent).await;
    assert_eq!(sessions[0]["additions_snapshot"]["flags"][0], "--second");
    assert_eq!(sessions[1]["exit_result"], "signal");
    client.ask("agent.stop", json!({"agent_id": agent})).await;
    office.serving.stop().await;
}

// (f): the CLI's own configuration is never written.

/// File list, size, modification time, and a hash of the content, for every
/// file under the folders.
fn snapshot(folders: &[PathBuf]) -> BTreeMap<PathBuf, (u64, SystemTime, u64)> {
    let mut files = BTreeMap::new();
    for folder in folders {
        collect(folder, &mut files);
    }
    files
}

fn collect(folder: &Path, files: &mut BTreeMap<PathBuf, (u64, SystemTime, u64)>) {
    for entry in fs::read_dir(folder).expect("folder").flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.insert(path.clone(), (0, SystemTime::UNIX_EPOCH, 0));
            collect(&path, files);
            continue;
        }
        let bytes = fs::read(&path).expect("read");
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let modified = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .expect("mtime");
        files.insert(path, (bytes.len() as u64, modified, hasher.finish()));
    }
}

fn plant_fake_configuration(clis: &FakeClis) {
    let home = clis.home_folder();
    let [config, data, _, _] = clis.xdg_folders();
    for (path, content) in [
        (
            home.join(".claude").join("settings.json"),
            "{\"theme\":\"dark\"}",
        ),
        (home.join(".codex").join("config.toml"), "model = \"x\""),
        (
            home.join(".codex")
                .join("sessions")
                .join("2026")
                .join("01")
                .join("01")
                .join("rollout-old.jsonl"),
            "{}",
        ),
        (home.join(".gemini").join("settings.json"), "{}"),
        (config.join("opencode").join("opencode.json"), "{}"),
        (data.join("opencode").join("storage.json"), "{}"),
    ] {
        fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        fs::write(path, content).expect("file");
    }
}

#[tokio::test]
async fn create_start_continue_edit_stop_and_delete_change_nothing_in_the_cli_configuration() {
    let clis = FakeClis::new().with_attribution_timeout(SHORT_ATTRIBUTION);
    plant_fake_configuration(&clis);
    let watched: Vec<PathBuf> = std::iter::once(clis.home_folder())
        .chain(clis.xdg_folders())
        .collect();
    let before = snapshot(&watched);
    let office = Office::open_with(clis).await;
    for cli in ["claude", "codex", "gemini", "opencode"] {
        office.clis.add(cli, &print_then_wait("quiet"));
    }
    let mut client = office.client().await;
    shorten_the_stop_grace(&mut client).await;

    for cli in ["claude", "codex", "gemini", "opencode"] {
        let agent = client
            .create_agent(&office, cli, cli, Some(first_additions()))
            .await;
        let started = client.start(&agent).await;
        client
            .ask(
                "agent.additions.set",
                json!({"agent_id": agent, "additions": second_additions()}),
            )
            .await;
        client.ask("agent.stop", json!({"agent_id": agent})).await;
        // Continue refuses when the session id is unknown (the fake `codex`
        // writes none); the attempt itself must still leave no trace.
        client
            .ask("session.continue", json!({"session_id": started["id"]}))
            .await;
        client.ask("agent.stop", json!({"agent_id": agent})).await;
        let deleted = client.ask("agent.delete", json!({"agent_id": agent})).await;
        assert_eq!(deleted["ok"], true, "{deleted}");
        assert_eq!(snapshot(&watched), before, "after the steps for {cli}");
    }
    tokio::time::sleep(SHORT_ATTRIBUTION + Duration::from_millis(300)).await;
    assert_eq!(
        snapshot(&watched),
        before,
        "after the attribution windows closed"
    );
    office.serving.stop().await;
}
