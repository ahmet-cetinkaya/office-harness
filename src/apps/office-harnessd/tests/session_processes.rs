// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]
#![cfg(unix)]

//! The recorder of 0017: the identities of the CLI and of its helpers are
//! stored while a session runs, kept while a helper runs, updated when a
//! process replaces itself, capped at the helper limit, and ended with the
//! session. The fake CLIs are `sh` scripts.

mod support;

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use agent_application::common::session_process_recorder::MAX_OPEN_HELPERS_PER_SESSION;
use office_harnessd::config::Config;
use rusqlite::Connection;
use serde_json::json;
use support::fake_cli::FakeClis;
use support::office_client::{Client, Office, wait_for};
use support::server_process::{connect, spawn_server, wait_until_started};

/// The limit of 0017/FR-002 for a new process to be stored.
const STORE_LIMIT: Duration = Duration::from_secs(10);
/// Two scan periods and a little more: a helper that was dropped wrongly
/// would be gone by then.
const TWO_SCANS_AND_A_LITTLE: Duration = Duration::from_secs(11);
const POLL: Duration = Duration::from_millis(100);
const FAKE_HELPERS: usize = 500;
const PRIVATE_FILE_MODE: u32 = 0o600;

/// A CLI that starts one long-lived helper and waits.
const HELPER_CLI: &str = "helper-cli";
/// A CLI that starts a helper through a middle shell, so killing the middle
/// shell moves the helper out of the CLI's process tree. The CLI keeps
/// running when the middle shell is gone.
const DETACHING_CLI: &str = "detaching-cli";
/// A CLI whose helper replaces itself with `sleep 600` after 7 seconds.
const EXEC_CLI: &str = "exec-cli";
const LIMIT_CLI: &str = "limit-cli";

#[derive(Debug, Clone)]
struct StoredRow {
    id: i64,
    pid: u32,
    kind: String,
    state: String,
    display: String,
}

fn rows_of(config: &Config, session_id: &str) -> Vec<StoredRow> {
    let connection = Connection::open(config.database_path()).expect("database");
    let mut statement = connection
        .prepare(
            "SELECT id, pid, kind, state, command_display FROM agent_session_processes \
             WHERE session_id = ?1 ORDER BY id",
        )
        .expect("prepare");
    statement
        .query_map([session_id], |row| {
            Ok(StoredRow {
                id: row.get(0)?,
                pid: row.get(1)?,
                kind: row.get(2)?,
                state: row.get(3)?,
                display: row.get(4)?,
            })
        })
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows")
}

fn open_helper_count(config: &Config, session_id: &str) -> usize {
    rows_of(config, session_id)
        .iter()
        .filter(|row| row.kind == "helper" && row.state == "open")
        .count()
}

fn count_in_office(config: &Config, sql: &str, parameter: &str) -> i64 {
    Connection::open(config.database_path())
        .expect("database")
        .query_row(sql, [parameter], |row| row.get(0))
        .expect("count")
}

fn read_pid(records: &Path, name: &str) -> Option<u32> {
    std::fs::read_to_string(records.join(name))
        .ok()?
        .trim()
        .parse()
        .ok()
}

fn is_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|status| status.success())
}

fn kill_hard(pid: u32) {
    let _ = Command::new("kill")
        .args(["-KILL", &pid.to_string()])
        .status();
}

fn parent_of(pid: u32) -> u32 {
    let output = Command::new("ps")
        .args(["-o", "ppid=", "-p", &pid.to_string()])
        .output()
        .expect("ps");
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .expect("a parent id")
}

/// The fake CLIs of these tests. Each writes its own pid, and the pid of the
/// processes it starts, into `records`.
fn add_clis(clis: &FakeClis, records: &Path) {
    let records = records.display();
    clis.add(
        HELPER_CLI,
        &format!(
            "printf '%s' \"$$\" > {records}/pid-cli\nsleep 600 &\nprintf '%s' \"$!\" > {records}/pid-helper\necho ready\nwait"
        ),
    );
    clis.add(
        DETACHING_CLI,
        &format!(
            "printf '%s' \"$$\" > {records}/pid-cli\nsh -c 'printf \"%s\" \"$$\" > {records}/pid-middle; sleep 600 & printf \"%s\" \"$!\" > {records}/pid-helper; wait' &\necho ready\nwhile :; do sleep 1; done"
        ),
    );
    clis.add(
        EXEC_CLI,
        &format!(
            "printf '%s' \"$$\" > {records}/pid-cli\nsh -c 'printf \"%s\" \"$$\" > {records}/pid-exec; sleep 7; exec sleep 600' &\necho ready\nwait"
        ),
    );
    clis.add(
        LIMIT_CLI,
        &format!(
            "printf '%s' \"$$\" > {records}/pid-cli\ni=0\nwhile [ $i -lt {FAKE_HELPERS} ]; do sleep 600 & i=$((i+1)); done\necho ready\nwait"
        ),
    );
}

async fn office_with_clis() -> Office {
    let clis = FakeClis::new();
    let office = Office::open_with(clis).await;
    add_clis(&office.clis, office.records_path());
    office
}

/// Starts `command` as a new agent and returns the agent id and session id.
async fn start_session(office: &Office, client: &mut Client, command: &str) -> (String, String) {
    let agent = client.create_agent(office, "Ana", command, None).await;
    let session = client.start(&agent).await;
    (
        agent,
        session["id"].as_str().expect("session id").to_owned(),
    )
}

async fn wait_for_row(
    config: &Config,
    session_id: &str,
    what: &str,
    mut wanted: impl FnMut(&StoredRow) -> bool,
) -> StoredRow {
    wait_for(what, || {
        rows_of(config, session_id)
            .into_iter()
            .find(|row| wanted(row))
    })
    .await
}

/// Waits for a row of `pid` and fails when it takes longer than the 10
/// seconds of 0017/FR-002, counted from now.
async fn row_within_store_limit(
    config: &Config,
    session_id: &str,
    pid: u32,
    kind: &str,
) -> StoredRow {
    let deadline = Instant::now() + STORE_LIMIT;
    loop {
        let found = rows_of(config, session_id)
            .into_iter()
            .find(|row| row.pid == pid && row.kind == kind);
        if let Some(row) = found {
            return row;
        }
        assert!(
            Instant::now() < deadline,
            "no {kind} row for {pid} within {STORE_LIMIT:?}"
        );
        tokio::time::sleep(POLL).await;
    }
}

#[tokio::test]
async fn the_cli_and_a_helper_are_stored_within_ten_seconds() {
    let office = office_with_clis().await;
    let mut client = office.client().await;
    let (agent, session) = start_session(&office, &mut client, HELPER_CLI).await;
    let config = &office.serving.office.config;
    let cli_pid = wait_for("the CLI pid", || read_pid(office.records_path(), "pid-cli")).await;
    let helper_pid = wait_for("the helper pid", || {
        read_pid(office.records_path(), "pid-helper")
    })
    .await;

    let cli = row_within_store_limit(config, &session, cli_pid, "cli").await;
    let helper = row_within_store_limit(config, &session, helper_pid, "helper").await;

    assert_eq!(cli.state, "open");
    assert_eq!(helper.state, "open");
    assert!(helper.display.starts_with("sleep 600"), "{helper:?}");
    client.ask("agent.stop", json!({"agent_id": agent})).await;
}

#[tokio::test]
async fn a_helper_that_left_the_cli_tree_stays_stored_while_it_runs() {
    let office = office_with_clis().await;
    let mut client = office.client().await;
    let (agent, session) = start_session(&office, &mut client, DETACHING_CLI).await;
    let config = &office.serving.office.config;
    let cli_pid = wait_for("the CLI pid", || read_pid(office.records_path(), "pid-cli")).await;
    let middle_pid = wait_for("the middle pid", || {
        read_pid(office.records_path(), "pid-middle")
    })
    .await;
    let helper_pid = wait_for("the helper pid", || {
        read_pid(office.records_path(), "pid-helper")
    })
    .await;
    row_within_store_limit(config, &session, helper_pid, "helper").await;

    kill_hard(middle_pid);
    tokio::time::sleep(TWO_SCANS_AND_A_LITTLE).await;

    assert_ne!(
        parent_of(helper_pid),
        cli_pid,
        "the helper is not a child of the CLI any more"
    );
    assert!(is_alive(helper_pid));
    let helper = rows_of(config, &session)
        .into_iter()
        .find(|row| row.pid == helper_pid)
        .expect("the helper row is kept");
    assert_eq!(helper.state, "open");
    client.ask("agent.stop", json!({"agent_id": agent})).await;
}

#[tokio::test]
async fn a_stop_ends_every_stored_identity_of_the_session() {
    let office = office_with_clis().await;
    let mut client = office.client().await;
    let (agent, session) = start_session(&office, &mut client, HELPER_CLI).await;
    let config = &office.serving.office.config;
    let cli_pid = wait_for("the CLI pid", || read_pid(office.records_path(), "pid-cli")).await;
    let helper_pid = wait_for("the helper pid", || {
        read_pid(office.records_path(), "pid-helper")
    })
    .await;
    row_within_store_limit(config, &session, cli_pid, "cli").await;
    row_within_store_limit(config, &session, helper_pid, "helper").await;

    let stopped = client.ask("agent.stop", json!({"agent_id": agent})).await;

    assert_eq!(stopped["ok"], true, "{stopped}");
    let rows = rows_of(config, &session);
    assert!(rows.len() >= 2, "{rows:?}");
    assert!(rows.iter().all(|row| row.state == "ended"), "{rows:?}");
}

#[tokio::test]
async fn a_process_that_replaces_itself_takes_the_new_command_without_a_second_row() {
    let office = office_with_clis().await;
    let mut client = office.client().await;
    let (agent, session) = start_session(&office, &mut client, EXEC_CLI).await;
    let config = &office.serving.office.config;
    let exec_pid = wait_for("the exec pid", || {
        read_pid(office.records_path(), "pid-exec")
    })
    .await;
    let before = row_within_store_limit(config, &session, exec_pid, "helper").await;

    let after = wait_for_row(config, &session, "the replaced command line", |row| {
        row.pid == exec_pid && row.display.starts_with("sleep 600")
    })
    .await;

    assert_eq!(after.id, before.id, "the row is updated in place");
    assert_eq!(after.state, "open");
    let rows_of_pid = rows_of(config, &session)
        .into_iter()
        .filter(|row| row.pid == exec_pid)
        .count();
    assert_eq!(rows_of_pid, 1);
    let log = office.serving.office.log_text();
    assert!(!log.contains("UNIQUE"), "no unique key failure: {log}");
    client.ask("agent.stop", json!({"agent_id": agent})).await;
}

#[tokio::test]
async fn a_session_stores_at_most_the_helper_limit_and_writes_one_entry() {
    let office = office_with_clis().await;
    let mut client = office.client().await;
    let (agent, session) = start_session(&office, &mut client, LIMIT_CLI).await;
    let config = &office.serving.office.config;
    let entries_sql = "SELECT COUNT(*) FROM audit_actions \
                       WHERE target_agent_id = ?1 AND action = 'helper_limit'";
    wait_for("the helper limit", || {
        (open_helper_count(config, &session) == MAX_OPEN_HELPERS_PER_SESSION
            && count_in_office(config, entries_sql, &agent) == 1)
            .then_some(())
    })
    .await;

    tokio::time::sleep(TWO_SCANS_AND_A_LITTLE).await;

    assert_eq!(
        open_helper_count(config, &session),
        MAX_OPEN_HELPERS_PER_SESSION
    );
    assert_eq!(count_in_office(config, entries_sql, &agent), 1);
    let marked = count_in_office(
        config,
        "SELECT helpers_capped FROM agent_sessions WHERE id = ?1",
        &session,
    );
    assert_eq!(marked, 1);
    client.ask("agent.stop", json!({"agent_id": agent})).await;
}

#[tokio::test]
async fn the_office_database_stays_private_to_its_owner_while_identities_are_stored() {
    let office = office_with_clis().await;
    let mut client = office.client().await;
    let (agent, session) = start_session(&office, &mut client, HELPER_CLI).await;
    let config = &office.serving.office.config;
    let cli_pid = wait_for("the CLI pid", || read_pid(office.records_path(), "pid-cli")).await;
    row_within_store_limit(config, &session, cli_pid, "cli").await;

    let mode = std::fs::metadata(config.database_path())
        .expect("database file")
        .permissions()
        .mode()
        & 0o777;

    assert_eq!(mode, PRIVATE_FILE_MODE);
    client.ask("agent.stop", json!({"agent_id": agent})).await;
}

#[tokio::test]
async fn a_clean_server_restart_ends_every_stored_identity() {
    let folder = tempfile::tempdir().expect("folder");
    let work = tempfile::tempdir().expect("work folder");
    let records = tempfile::tempdir().expect("records");
    let config = support::config_in(&folder);
    let clis = FakeClis::new();
    let _ = clis.login_setup();
    add_clis(&clis, records.path());
    let mut server = spawn_server(&config, &clis);
    wait_until_started(&config, 1).await;
    let mut client = connect(&config).await;
    let created = client
        .ask(
            "agent.create",
            json!({"name": "Ana", "command": HELPER_CLI, "working_folder": work.path()}),
        )
        .await;
    let agent = created["result"]["id"].as_str().expect("id").to_owned();
    let session = client.start(&agent).await["id"]
        .as_str()
        .expect("session id")
        .to_owned();
    let cli_pid = wait_for("the CLI pid", || read_pid(records.path(), "pid-cli")).await;
    let helper_pid = wait_for("the helper pid", || read_pid(records.path(), "pid-helper")).await;
    row_within_store_limit(&config, &session, cli_pid, "cli").await;
    row_within_store_limit(&config, &session, helper_pid, "helper").await;

    let answer = client.ask("server.restart", json!({})).await;

    assert_eq!(answer["ok"], true, "{answer}");
    wait_for("the server to exit", || server.0.try_wait().expect("poll")).await;
    let rows = rows_of(&config, &session);
    assert!(rows.len() >= 2, "{rows:?}");
    assert!(rows.iter().all(|row| row.state == "ended"), "{rows:?}");
}
