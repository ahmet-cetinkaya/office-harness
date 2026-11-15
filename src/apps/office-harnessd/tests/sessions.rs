// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]
#![cfg(unix)]

//! T7 at protocol level: office additions, sessions, and the CLI adapters,
//! through a real socket, with fake CLIs started through the whole server.

mod support;

use std::collections::BTreeMap;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde_json::{Value, json};
use support::fake_cli::FakeClis;
use support::office_client::{
    Office, error_text, recording_body, records_of, wait_for, wait_for_records,
};

const BRACKETED_PASTE_ON: &str = r"printf '\033[?2004h'";
const PASTE_START: &str = "\u{1b}[200~";
const PASTE_END: &str = "\u{1b}[201~";
const SNAPSHOT_HEADER_BYTES: usize = 17;
const SHORT_ATTRIBUTION: Duration = Duration::from_millis(600);

fn first_additions() -> Value {
    json!({
        "flags": ["--first"],
        "env": [{"name": "OFFICE_KEY", "value": "one"}],
        "startup_prompt": "prompt one",
    })
}

fn second_additions() -> Value {
    json!({
        "flags": ["--second"],
        "env": [{"name": "OFFICE_KEY", "value": "two"}],
        "startup_prompt": "prompt two",
    })
}

// (a), (c): additions reach the child at the next session only.

#[tokio::test]
async fn flags_env_and_the_startup_prompt_reach_the_child_at_the_next_session_and_an_edit_leaves_the_running_one()
 {
    let office = Office::open().await;
    office.clis.add(
        "claude",
        &recording_body(office.records_path(), "echo ready; sleep 30"),
    );
    let mut client = office.client().await;
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
    let sessions = client.sessions(&agent).await;
    assert_eq!(sessions[0]["additions_snapshot"]["flags"][0], "--second");
    assert_eq!(sessions[1]["exit_result"], "signal");
    client.ask("agent.stop", json!({"agent_id": agent})).await;
    office.serving.stop().await;
}

#[tokio::test]
async fn a_malformed_environment_name_is_refused_and_named_at_create_and_at_edit() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let bad = json!({"env": [{"name": "1BAD", "value": "x"}]});

    let at_create = client
        .ask(
            "agent.create",
            json!({"name": "Ana", "command": "claude", "working_folder": office.work_path(), "additions": bad}),
        )
        .await;
    let agent = client.create_agent(&office, "Ben", "claude", None).await;
    let at_edit = client
        .ask(
            "agent.additions.set",
            json!({"agent_id": agent, "additions": bad}),
        )
        .await;
    let wrong_shape = client
        .ask(
            "agent.additions.set",
            json!({"agent_id": agent, "additions": {"env": [{"name": 5, "value": "x"}]}}),
        )
        .await;

    for refused in [&at_create, &at_edit] {
        assert_eq!(error_text(refused).0, "invalid", "{refused}");
        assert!(error_text(refused).1.contains("\"1BAD\""), "{refused}");
    }
    assert!(error_text(&wrong_shape).1.contains("additions.env[0].name"));
    let stored = client
        .ask("agent.additions.get", json!({"agent_id": agent}))
        .await;
    assert_eq!(stored["result"]["env"], json!([]));
    let agents = client.ask("agent.list", json!({})).await;
    assert_eq!(
        agents["result"].as_array().expect("agents").len(),
        1,
        "no agent for the refused create"
    );
    office.serving.stop().await;
}

// (b): continue.

#[tokio::test]
async fn continuing_a_claude_session_resumes_the_old_id_without_the_prompt_and_records_the_origin()
{
    let office = Office::open().await;
    office.clis.add(
        "claude",
        &recording_body(office.records_path(), "echo ready"),
    );
    let mut client = office.client().await;
    let agent = client
        .create_agent(&office, "Ana", "claude", Some(first_additions()))
        .await;
    let first = client.start(&agent).await;
    client.wait_for_status(&agent, "exited").await;
    let listed = client.sessions(&agent).await;
    assert_eq!(listed[0]["can_continue"], true, "{listed:?}");

    let continued = client
        .ask("session.continue", json!({"session_id": first["id"]}))
        .await;

    assert_eq!(continued["ok"], true, "{continued}");
    let cli_id = first["cli_session_id"].as_str().expect("cli id");
    let records = wait_for_records(office.records_path(), 2).await;
    assert_eq!(records[1].arguments, ["--first", "--resume", cli_id]);
    assert_eq!(continued["result"]["continued_from"], first["id"]);
    assert_eq!(continued["result"]["cli_session_id"], cli_id);
    let actions = client
        .ask("agent.actions", json!({"agent_id": agent}))
        .await;
    let kinds: Vec<&str> = actions["result"]
        .as_array()
        .expect("entries")
        .iter()
        .filter_map(|entry| entry["action"].as_str())
        .collect();
    assert_eq!(kinds, ["create", "start", "continue"]);
    office.serving.stop().await;
}

#[tokio::test]
async fn a_generic_command_cannot_continue_and_the_refusal_gives_the_reason() {
    let office = Office::open().await;
    office.clis.add("plain-cli", "echo hi");
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "plain-cli", None).await;
    let started = client.start(&agent).await;
    client.wait_for_status(&agent, "exited").await;

    let sessions = client.sessions(&agent).await;
    let refused = client
        .ask("session.continue", json!({"session_id": started["id"]}))
        .await;

    assert_eq!(sessions[0]["can_continue"], false);
    assert!(
        sessions[0]["reason"]
            .as_str()
            .expect("reason")
            .contains("does not support")
    );
    assert_eq!(error_text(&refused).0, "refused");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_stopped_agent_attaches_to_the_output_of_its_last_session() {
    let office = Office::open().await;
    office.clis.add("claude", "echo saved-output-marker");
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "claude", None).await;
    client.start(&agent).await;
    client.wait_for_status(&agent, "exited").await;
    client.start(&agent).await;
    client.wait_for_status(&agent, "exited").await;

    let attached = client
        .ask(
            "terminal.attach",
            json!({"agent_id": agent, "cols": 80, "rows": 24}),
        )
        .await;

    assert_eq!(attached["ok"], true, "{attached}");
    let frame = client.wire.receive_binary().await;
    let text = String::from_utf8_lossy(&frame[SNAPSHOT_HEADER_BYTES..]).into_owned();
    assert!(text.contains("saved-output-marker"), "{text}");
    let output_files = fs::read_dir(office.serving.office.config.output_folder_path())
        .expect("output folder")
        .count();
    assert_eq!(output_files, 2, "one output file for each session");
    office.serving.stop().await;
}

// (d): codex attribution.

/// A fake `codex` that writes its session file as the real one does, in
/// `$HOME/.codex/sessions`, after `delay` seconds, and tells the test its id.
fn fake_codex_body(office: &Office, delay: &str) -> String {
    let records = office.records_path().display();
    format!(
        "tag=\"$2\"\nid=\"$(printf '00000000-0000-0000-0000-%012d' $$)\"\nsleep {delay}\nday=\"$HOME/.codex/sessions/2026/11/15\"\nmkdir -p \"$day\"\n\
         printf '{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"%s\",\"cwd\":\"%s\"}}}}\\n' \"$id\" \"$PWD\" > \"$day/rollout-$id.jsonl\"\n\
         printf '%s' \"$id\" > {records}/own-$tag\nsleep 30"
    )
}

fn own_id(office: &Office, tag: &str) -> Option<String> {
    fs::read_to_string(office.records_path().join(format!("own-{tag}"))).ok()
}

#[tokio::test]
async fn two_codex_agents_in_one_folder_never_get_each_others_id() {
    let office = Office::open().await;
    office.clis.add("codex", &fake_codex_body(&office, "0.4"));
    let mut client = office.client().await;
    let ana = client
        .create_agent(&office, "Ana", "codex --tag ana", None)
        .await;
    let ben = client
        .create_agent(&office, "Ben", "codex --tag ben", None)
        .await;
    let first = client.send("agent.start", json!({"agent_id": ana})).await;
    let second = client.send("agent.start", json!({"agent_id": ben})).await;
    client.answer_of(first).await;
    client.answer_of(second).await;
    let ana_id = wait_for("Ana's session file", || own_id(&office, "ana")).await;
    let ben_id = wait_for("Ben's session file", || own_id(&office, "ben")).await;
    tokio::time::sleep(SHORT_ATTRIBUTION + Duration::from_millis(400)).await;

    for (agent, own, other) in [(&ana, &ana_id, &ben_id), (&ben, &ben_id, &ana_id)] {
        let sessions = client.sessions(agent).await;
        let cli_id = sessions[0]["cli_session_id"].as_str();
        assert_ne!(cli_id, Some(other.as_str()), "never the other agent's id");
        if let Some(id) = cli_id {
            assert_eq!(id, own);
        } else {
            assert_eq!(sessions[0]["can_continue"], false);
            assert!(
                sessions[0]["reason"]
                    .as_str()
                    .is_some_and(|reason| !reason.is_empty())
            );
        }
    }
    for agent in [&ana, &ben] {
        client.ask("agent.stop", json!({"agent_id": agent})).await;
    }
    office.serving.stop().await;
}

#[tokio::test]
async fn a_lone_codex_agent_gets_its_session_id_and_continues_it_with_resume() {
    let office = Office::open().await;
    office.clis.add("codex", &fake_codex_body(&office, "0.1"));
    let mut client = office.client().await;
    let agent = client
        .create_agent(&office, "Ana", "codex --tag ana", None)
        .await;
    let started = client.start(&agent).await;
    let own = wait_for("the session file", || own_id(&office, "ana")).await;
    let found = wait_for_session_id(&mut client, &agent).await;
    assert_eq!(found, own);
    client.ask("agent.stop", json!({"agent_id": agent})).await;

    let continued = client
        .ask("session.continue", json!({"session_id": started["id"]}))
        .await;

    assert_eq!(continued["ok"], true, "{continued}");
    assert_eq!(continued["result"]["cli_session_id"], own.as_str());
    client.ask("agent.stop", json!({"agent_id": agent})).await;
    office.serving.stop().await;
}

async fn wait_for_session_id(client: &mut support::office_client::Client, agent: &str) -> String {
    for _ in 0..200 {
        let sessions = client.sessions(agent).await;
        if let Some(id) = sessions[0]["cli_session_id"].as_str() {
            return id.to_owned();
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the session id was never found");
}

// (e): the startup prompt, pasted and never submitted.

const SHELL_PROMPT: &str = "echo %PATH% & calc\nsecond line";

/// A fake `.cmd` CLI that reads exactly `count` typed bytes without echo and
/// writes them to `typed`. It sets the raw mode before its first output, so
/// the typing that follows the first output finds the mode ready.
fn typing_cli(office: &Office, count: usize, paste_mode: &str) -> (String, PathBuf) {
    let typed = office.records_path().join("typed");
    let body = format!(
        "stty raw -echo\n{paste_mode}\ndd bs=1 count={count} of={} 2>/dev/null\nsleep 30",
        typed.display()
    );
    (body, typed)
}

async fn typed_bytes_for(paste_mode: &str, expected: &str) {
    let office = Office::open().await;
    let (body, typed) = typing_cli(&office, expected.len(), paste_mode);
    office.clis.add("shim.cmd", &body);
    let mut client = office.client().await;
    let additions = json!({"startup_prompt": SHELL_PROMPT});
    let agent = client
        .create_agent(&office, "Ana", "shim.cmd", Some(additions))
        .await;

    client.start(&agent).await;

    let received = wait_for("the typed bytes", || {
        fs::read(&typed)
            .ok()
            .filter(|bytes| bytes.len() == expected.len())
    })
    .await;
    assert_eq!(String::from_utf8_lossy(&received), expected);
    client.ask("agent.stop", json!({"agent_id": agent})).await;
    office.serving.stop().await;
}

#[tokio::test]
async fn a_startup_prompt_with_shell_characters_reaches_a_batch_cli_byte_for_byte_inside_a_paste() {
    let expected = format!("{PASTE_START}{SHELL_PROMPT}{PASTE_END}");

    typed_bytes_for(BRACKETED_PASTE_ON, &expected).await;
}

#[tokio::test]
async fn without_the_paste_mode_the_prompt_is_typed_plainly() {
    let expected = SHELL_PROMPT.to_owned();

    typed_bytes_for("echo ready", &expected).await;
}

#[tokio::test]
async fn a_flag_or_a_fixed_argument_with_a_percent_is_refused_for_a_batch_cli() {
    let office = Office::open().await;
    office.clis.add("shim.cmd", "echo hi");
    let mut client = office.client().await;
    let by_flag = client
        .create_agent(
            &office,
            "Ana",
            "shim.cmd",
            Some(json!({"flags": ["--path=%PATH%"]})),
        )
        .await;
    let by_fixed = client
        .create_agent(&office, "Ben", "shim.cmd --x=%PATH%", None)
        .await;

    for agent in [&by_flag, &by_fixed] {
        let refused = client.ask("agent.start", json!({"agent_id": agent})).await;

        assert_eq!(error_text(&refused).0, "invalid", "{refused}");
        assert!(error_text(&refused).1.contains('%'), "{refused}");
        assert_eq!(client.status_of(agent).await, "stopped");
        assert_eq!(client.sessions(agent).await.len(), 0);
    }
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
        office.clis.add(cli, "echo quiet; sleep 30");
    }
    let mut client = office.client().await;

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
