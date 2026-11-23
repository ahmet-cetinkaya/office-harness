// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! The cleanup of 0023 at each server start: the identities that the rules do
//! not keep are deleted, the write log is truncated, and a one-time rebuild
//! runs until it has finished (FR-002 to FR-006).

mod support;

use std::fs;

use agent_domain::value_objects::SessionId;
use kernel::AgentId;
use office_harnessd::bootstrap::{self, Services};
use office_harnessd::config::Config;
use rusqlite::{Connection, params};
use support::{agent_named, config_in};

const REBUILT_LINE: &str = "office database rebuilt";
const TRUNCATE_FAILED_LINE: &str = "could not truncate the write log";
const MARK_TASK: &str = "freed_text_cleared";
const EMPTY_ADDITIONS: &str = r#"{"flags":[],"env":[]}"#;
const NORMAL_END: &str = "0";
const LOST_END: &str = "lost";
/// Enough rows that the write log of the seed holds many pages.
const BULK_ROWS: usize = 300;
const BULK_TEXT_LENGTH: usize = 900;

fn start_server_state(config: &Config) -> Services {
    let log = bootstrap::open_log(config).expect("log");
    bootstrap::start(config, &log).expect("start")
}

fn open_office(config: &Config) -> Connection {
    Connection::open(config.database_path()).expect("database")
}

fn log_lines_with(config: &Config, text: &str) -> usize {
    fs::read_to_string(config.log_path())
        .expect("log")
        .lines()
        .filter(|line| line.contains(text))
        .count()
}

fn mark_rows(config: &Config) -> Vec<i64> {
    open_office(config)
        .prepare("SELECT done_at FROM agent_maintenance WHERE task = ?1")
        .expect("prepare")
        .query_map([MARK_TASK], |row| row.get(0))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows")
}

fn remove_mark(config: &Config) {
    open_office(config)
        .execute("DELETE FROM agent_maintenance", [])
        .expect("remove mark");
}

fn write_log_path(config: &Config) -> std::path::PathBuf {
    let mut name = config.database_path().into_os_string();
    name.push("-wal");
    name.into()
}

fn write_log_length(config: &Config) -> u64 {
    fs::metadata(write_log_path(config)).map_or(0, |metadata| metadata.len())
}

/// A database that one start created, with one agent, ready to be seeded.
struct Office {
    _folder: tempfile::TempDir,
    config: Config,
    agent: AgentId,
}

impl Office {
    fn started_once() -> Self {
        let folder = tempfile::tempdir().expect("folder");
        let config = config_in(&folder);
        let first = start_server_state(&config);
        let agent = agent_named(first.machine.id, "Ana");
        first.agents.insert(&agent).expect("agent");
        Self {
            _folder: folder,
            agent: agent.id,
            config,
        }
    }

    /// A session of the agent with one ended and one open row, when
    /// `with_open_row` is set. Returns the session id.
    fn seed_session(
        &self,
        connection: &Connection,
        started_at: i64,
        exit_result: &str,
        with_open_row: bool,
    ) -> String {
        let session_id = SessionId::generate().to_string();
        connection
            .execute(
                "INSERT INTO agent_sessions \
                 (id, agent_id, additions_snapshot, started_at, ended_at, exit_result) \
                 VALUES (?1, ?2, ?3, ?4, ?4, ?5)",
                params![
                    session_id,
                    self.agent.to_string(),
                    EMPTY_ADDITIONS,
                    started_at,
                    exit_result
                ],
            )
            .expect("session");
        Self::seed_row(connection, &session_id, 1, "ended");
        if with_open_row {
            Self::seed_row(connection, &session_id, 2, "open");
        }
        session_id
    }

    fn seed_row(connection: &Connection, session_id: &str, pid: i64, state: &str) {
        connection
            .execute(
                "INSERT INTO agent_session_processes \
                 (session_id, pid, started_at, command_hash, command_display, boot, kind, state) \
                 VALUES (?1, ?2, 1, zeroblob(32), 'secret command', 'boot', 'cli', ?3)",
                params![session_id, pid, state],
            )
            .expect("process row");
    }

    fn state_rows_of(&self, session_id: &str) -> Vec<String> {
        open_office(&self.config)
            .prepare("SELECT state FROM agent_session_processes WHERE session_id = ?1 ORDER BY pid")
            .expect("prepare")
            .query_map([session_id], |row| row.get(0))
            .expect("query")
            .collect::<Result<_, _>>()
            .expect("rows")
    }
}

#[test]
fn a_start_deletes_the_ended_rows_that_the_rules_do_not_keep_and_keeps_the_rest() {
    let office = Office::started_once();
    let connection = open_office(&office.config);
    let normal = office.seed_session(&connection, 100, NORMAL_END, true);
    let earlier_lost = office.seed_session(&connection, 200, LOST_END, true);
    let last_lost = office.seed_session(&connection, 300, LOST_END, true);
    drop(connection);

    let _second = start_server_state(&office.config);

    assert_eq!(office.state_rows_of(&normal), Vec::<String>::new());
    assert_eq!(office.state_rows_of(&earlier_lost), ["open"]);
    assert_eq!(office.state_rows_of(&last_lost), ["ended", "open"]);
}

#[test]
fn a_start_truncates_the_write_log() {
    let office = Office::started_once();
    let connection = open_office(&office.config);
    connection
        .execute_batch("PRAGMA wal_autocheckpoint = 0")
        .expect("keep the log");
    for started_at in (100_i64..).take(BULK_ROWS) {
        let session = office.seed_session(&connection, started_at, NORMAL_END, false);
        connection
            .execute(
                "UPDATE agent_session_processes SET command_display = ?2 WHERE session_id = ?1",
                params![session, "x".repeat(BULK_TEXT_LENGTH)],
            )
            .expect("long text");
    }
    let length_before = write_log_length(&office.config);

    let _second = start_server_state(&office.config);

    let length_after = write_log_length(&office.config);
    assert!(
        length_after < length_before / 4,
        "write log {length_before} bytes before the start, {length_after} after"
    );
}

#[test]
fn the_rebuild_mark_is_written_once_and_the_next_start_skips_the_rebuild() {
    let office = Office::started_once();
    let marks_after_first = mark_rows(&office.config);
    assert_eq!(marks_after_first.len(), 1);
    assert_eq!(log_lines_with(&office.config, REBUILT_LINE), 1);

    let _second = start_server_state(&office.config);

    assert_eq!(mark_rows(&office.config), marks_after_first);
    assert_eq!(log_lines_with(&office.config, REBUILT_LINE), 1);
}

#[test]
fn a_busy_checkpoint_leaves_no_mark_and_the_next_start_finishes() {
    let office = Office::started_once();
    remove_mark(&office.config);
    let reader = open_office(&office.config);
    reader
        .execute_batch("BEGIN; SELECT count(*) FROM agent_sessions")
        .expect("read transaction");
    let writer = open_office(&office.config);
    office.seed_session(&writer, 100, NORMAL_END, false);
    drop(writer);

    let busy_start = start_server_state(&office.config);

    assert_eq!(mark_rows(&office.config), Vec::<i64>::new());
    assert_eq!(log_lines_with(&office.config, TRUNCATE_FAILED_LINE), 1);
    drop(busy_start);
    reader.execute_batch("COMMIT").expect("end read");

    let _finished_start = start_server_state(&office.config);

    assert_eq!(mark_rows(&office.config).len(), 1);
    assert_eq!(log_lines_with(&office.config, TRUNCATE_FAILED_LINE), 1);
}
