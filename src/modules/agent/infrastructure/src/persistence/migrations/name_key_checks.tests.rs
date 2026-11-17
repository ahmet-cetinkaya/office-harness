// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

use super::check_name_key_ascii_groups;
use crate::persistence::migrations::{LEDGER_TABLE, SCRIPTS};
use rusqlite::Connection;

/// A database that the runner brought to `version_count`: the scripts of the
/// first `version_count` versions applied, with one ledger row each.
fn connection_at_version(version_count: usize) -> Connection {
    let connection = Connection::open_in_memory().expect("database");
    if version_count > 0 {
        connection
            .execute_batch(&format!(
                "CREATE TABLE {LEDGER_TABLE} (
                     version INTEGER PRIMARY KEY,
                     applied_at INTEGER NOT NULL DEFAULT (unixepoch('subsec') * 1000)
                 )"
            ))
            .expect("ledger");
    }
    for (index, script) in SCRIPTS.iter().enumerate().take(version_count) {
        let version = i64::try_from(index + 1).expect("small count");
        connection.execute_batch(script).expect("script");
        connection
            .execute(
                &format!("INSERT INTO {LEDGER_TABLE} (version) VALUES (?1)"),
                [version],
            )
            .expect("ledger row");
    }
    connection
}

fn add_project(connection: &Connection, id: &str, name: &str, name_key: &str) {
    connection
        .execute(
            "INSERT INTO agent_projects (id, name, name_key, folder) VALUES (?1, ?2, ?3, '/w')",
            [id, name, name_key],
        )
        .expect("project row");
}

fn add_agent(connection: &Connection, id: &str, name: &str, name_key: &str) {
    connection
        .execute(
            "INSERT INTO agent_machines (id, name) VALUES ('m1', 'host')
             ON CONFLICT (id) DO NOTHING",
            [],
        )
        .expect("machine row");
    connection
        .execute(
            "INSERT INTO agent_agents (id, name, name_key, command, machine_id, status, created_at)
             VALUES (?1, ?2, ?3, 'claude', 'm1', 'stopped', 1)",
            [id, name, name_key],
        )
        .expect("agent row");
}

fn ledger_version(connection: &Connection) -> i64 {
    connection
        .query_row(
            &format!("SELECT COALESCE(MAX(version), 0) FROM {LEDGER_TABLE}"),
            [],
            |row| row.get(0),
        )
        .expect("ledger")
}

#[test]
fn a_fresh_database_without_tables_passes_the_check() {
    let connection = connection_at_version(0);

    check_name_key_ascii_groups(&connection).expect("no tables, no rows");
}

#[test]
fn a_clean_database_at_version_three_passes_the_check() {
    let connection = connection_at_version(3);
    add_project(&connection, "p1", "Web", "web");
    add_agent(&connection, "a1", "Ana", "ana");

    check_name_key_ascii_groups(&connection).expect("no collision");
}

#[test]
fn a_colliding_project_group_names_both_rows_and_keeps_the_data() {
    let connection = connection_at_version(3);
    add_project(&connection, "p1", "Web", "web");
    add_project(&connection, "p2", "WEB", "WEB");

    let error = check_name_key_ascii_groups(&connection).expect_err("collision");

    assert!(error.contains("agent_projects"), "{error}");
    assert!(error.contains("p1") && error.contains("Web"), "{error}");
    assert!(error.contains("p2") && error.contains("WEB"), "{error}");
    assert!(error.contains("rename or delete"), "{error}");
    assert_eq!(ledger_version(&connection), 3, "the ledger must not move");
    let projects: i64 = connection
        .query_row("SELECT COUNT(*) FROM agent_projects", [], |row| row.get(0))
        .expect("count");
    assert_eq!(projects, 2, "the check must not delete rows");
}

#[test]
fn a_second_group_in_agent_agents_is_listed_with_the_first() {
    let connection = connection_at_version(3);
    add_project(&connection, "p1", "Web", "web");
    add_project(&connection, "p2", "WEB", "WEB");
    add_agent(&connection, "a1", "Ana", "ana");
    add_agent(&connection, "a2", "ANA", "ANA");

    let error = check_name_key_ascii_groups(&connection).expect_err("collision");

    assert!(error.contains("agent_projects"), "{error}");
    assert!(error.contains("agent_agents"), "{error}");
    assert!(error.contains("a1") && error.contains("Ana"), "{error}");
    assert!(error.contains("a2") && error.contains("ANA"), "{error}");
}

#[test]
fn keys_that_differ_outside_ascii_case_are_not_a_collision() {
    let connection = connection_at_version(3);
    add_project(&connection, "p1", "Ana", "äna");
    add_project(&connection, "p2", "ANA", "ÄNA");
    add_agent(&connection, "a1", "Äna", "äna");
    add_agent(&connection, "a2", "ÄNA", "ÄNA");

    check_name_key_ascii_groups(&connection).expect("non-ASCII case is not folded");
}

#[test]
fn rows_outside_the_colliding_groups_stay_out_of_the_report() {
    let connection = connection_at_version(3);
    add_project(&connection, "p1", "Web", "web");
    add_project(&connection, "p2", "WEB", "WEB");
    add_project(&connection, "p3", "Web Master", "web master");

    let error = check_name_key_ascii_groups(&connection).expect_err("collision");

    assert!(
        !error.contains("Web Master"),
        "unrelated rows stay out of the report: {error}"
    );
}

#[test]
fn the_version_three_database_still_migrates_after_the_check_passes() {
    let connection = connection_at_version(3);
    add_project(&connection, "p1", "Ana", "ana");
    add_agent(&connection, "a1", "Web", "web");

    check_name_key_ascii_groups(&connection).expect("clean");

    connection
        .execute_batch(SCRIPTS[3])
        .expect("version 4 index");
    connection
        .execute(
            "INSERT INTO agent_projects (id, name, name_key, folder)
             VALUES ('p2', 'ANA', 'ANA', '/w')",
            [],
        )
        .expect_err("the version 4 index refuses an ASCII case twin");
    connection
        .execute(
            "INSERT INTO agent_agents (id, name, name_key, command, machine_id, status, created_at)
             VALUES ('a2', 'Äna', 'Äna', 'claude', 'm1', 'stopped', 1)",
            [],
        )
        .expect("a non-ASCII case twin is still accepted");
}
