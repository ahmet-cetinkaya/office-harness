// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! 0024 T3: a `claude` session starts with the office settings file and its
//! secret; every other CLI starts as before; the file and the secret go with
//! the session; nothing of the CLI's own folders changes.

mod support;

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use access_application::common::interfaces::HookSecretStore;
use kernel::AgentId;
use serde_json::Value;
use support::fake_cli::FakeClis;
use support::office_client::{Client, Office, wait_for};

const QUIET_PERIOD_MS: u64 = 400;
const SETTINGS_OPTION: &str = "--settings";

/// A session that writes its arguments and the hook secret of its
/// environment to the records folder.
fn claude_script(records: &Path) -> String {
    let records = records.display();
    format!(
        "echo \"$@\" > {records}/args; printf '%s' \"$OFFICE_HARNESS_HOOK_SECRET\" > {records}/secret; \
         echo ready; sleep 60"
    )
}

async fn open_with(command: &str, script: impl Fn(&Path) -> String) -> (Office, Client, String) {
    open_with_version(command, "2.1.322", script).await
}

async fn open_with_version(
    command: &str,
    version: &str,
    script: impl Fn(&Path) -> String,
) -> (Office, Client, String) {
    let clis = FakeClis::new().with_cli_version(version);
    let office = Office::open_with_quiet_period(clis, QUIET_PERIOD_MS).await;
    office.clis.add(command, &script(office.records_path()));
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", command, None).await;
    (office, client, agent)
}

async fn recorded(office: &Office, name: &str) -> String {
    let path = office.records_path().join(name);
    wait_for("the CLI's record", || std::fs::read_to_string(&path).ok()).await
}

fn hook_files(office: &Office) -> Vec<PathBuf> {
    let folder = office.serving.office.config.hooks_folder_path();
    std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .collect()
}

fn settings_path(arguments: &str) -> Option<PathBuf> {
    let words: Vec<_> = arguments.split_whitespace().collect();
    let at = words.iter().position(|word| *word == SETTINGS_OPTION)?;
    words.get(at + 1).map(PathBuf::from)
}

fn session_of(file: &Path) -> String {
    file.file_stem()
        .and_then(|stem| stem.to_str())
        .expect("file name")
        .to_owned()
}

#[tokio::test]
async fn a_claude_session_gets_the_file_and_the_secret_and_loses_both_at_its_stop() {
    let (office, mut client, agent) = open_with("claude", claude_script).await;
    client.start(&agent).await;
    let arguments = wait_for("the arguments", || {
        std::fs::read_to_string(office.records_path().join("args")).ok()
    })
    .await;
    let file = settings_path(&arguments).expect("the settings option");
    assert_eq!(hook_files(&office), vec![file.clone()]);
    let text = std::fs::read_to_string(&file).expect("settings file");
    let parsed: Value = serde_json::from_str(&text).expect("json");
    assert_eq!(parsed.as_object().expect("object").len(), 1);
    assert!(parsed["hooks"]["Stop"].is_array());

    let secret = wait_for("the secret", || {
        std::fs::read_to_string(office.records_path().join("secret"))
            .ok()
            .filter(|secret| !secret.is_empty())
    })
    .await;
    assert!(!text.contains(&secret), "the file holds no secret");
    let session = session_of(&file);
    let secrets = &office.serving.office.services.access.hook_secrets;
    let agent_id = AgentId::from_str(&agent).expect("agent id");
    assert_eq!(secrets.authorize(&session, &secret), Some(agent_id));

    let database = std::fs::read(office.serving.office.config.database_path()).expect("database");
    assert!(
        !String::from_utf8_lossy(&database).contains(&secret),
        "the secret is not in the database"
    );
    assert!(!office.serving.office.log_text().contains(&secret));

    let stopped = client
        .ask("agent.stop", serde_json::json!({"agent_id": agent}))
        .await;
    assert_eq!(stopped["ok"], true, "{stopped}");
    client.wait_for_status(&agent, "stopped").await;
    wait_for("the file to go", || {
        hook_files(&office).is_empty().then_some(())
    })
    .await;
    assert_eq!(secrets.authorize(&session, &secret), None);
    office.serving.stop().await;
}

#[tokio::test]
async fn another_cli_gets_no_file_no_option_and_no_secret() {
    let (office, mut client, agent) = open_with("test-cli", claude_script).await;
    client.start(&agent).await;
    let arguments = recorded(&office, "args").await;
    assert_eq!(settings_path(&arguments), None, "{arguments}");
    assert_eq!(recorded(&office, "secret").await, "");
    assert_eq!(hook_files(&office), Vec::<PathBuf>::new());
    office.serving.stop().await;
}

#[tokio::test]
async fn the_switch_off_and_an_old_version_give_a_session_without_the_file() {
    let (office, mut client, agent) = open_with("claude", claude_script).await;
    client.set_setting("hook_status", "off").await;
    client.start(&agent).await;
    assert_eq!(settings_path(&recorded(&office, "args").await), None);
    assert_eq!(hook_files(&office), Vec::<PathBuf>::new());
    office.serving.stop().await;

    let (old, mut old_client, old_agent) =
        open_with_version("claude", "2.1.100", claude_script).await;
    old_client.start(&old_agent).await;
    assert_eq!(settings_path(&recorded(&old, "args").await), None);
    assert_eq!(hook_files(&old), Vec::<PathBuf>::new());
    old.serving.stop().await;
}

#[tokio::test]
async fn the_cli_folders_are_not_touched() {
    let (office, mut client, agent) = open_with("claude", claude_script).await;
    let home = office.clis.home_folder();
    let xdg = office.clis.xdg_folders();
    let mut folders: Vec<&Path> = vec![home.as_path()];
    folders.extend(xdg.iter().map(PathBuf::as_path));
    let existing: Vec<&Path> = folders
        .into_iter()
        .filter(|folder| folder.exists())
        .collect();
    let before = support::entries_under(&existing);
    client.start(&agent).await;
    recorded(&office, "args").await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(support::entries_under(&existing), before);
    office.serving.stop().await;
}

#[tokio::test]
async fn a_session_that_exits_or_fails_or_stops_with_the_server_leaves_no_file() {
    for (name, ending) in [("exit", "exit 0"), ("failure", "exit 3")] {
        let (office, mut client, agent) = open_with("claude", |records| {
            let records = records.display();
            format!("echo \"$@\" > {records}/args; sleep 1; {ending}")
        })
        .await;
        client.start(&agent).await;
        let arguments = recorded(&office, "args").await;
        let file = settings_path(&arguments).expect("the settings option");
        let session = session_of(&file);
        wait_for(name, || hook_files(&office).is_empty().then_some(())).await;
        let secrets = &office.serving.office.services.access.hook_secrets;
        assert_eq!(secrets.authorize(&session, "anything"), None);
        office.serving.stop().await;
    }

    let (office, mut client, agent) = open_with("claude", claude_script).await;
    client.start(&agent).await;
    recorded(&office, "args").await;
    assert_eq!(hook_files(&office).len(), 1);
    let services = &office.serving.office.services;
    office_harnessd::bootstrap::interrupt_running_agents(services, &office.serving.office.log)
        .await;
    assert_eq!(hook_files(&office), Vec::<PathBuf>::new());
    office.serving.stop().await;
}
