// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

use agent_infrastructure::persistence::{LEDGER_TABLE, PRE_MIGRATION_CHECKS, SCRIPTS};
use office_harnessd::bootstrap::{StartupError, run_migrations};
use rusqlite::Connection;

fn schema_dump(connection: &Connection) -> Vec<(String, String)> {
    let mut statement = connection
        .prepare("SELECT name, sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY name")
        .expect("prepare");
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows")
}

/// The ledger versions after every script ran: 1, 2, and so on.
fn all_versions() -> Vec<i64> {
    (1..).take(SCRIPTS.len()).collect()
}

fn ledger_versions(connection: &Connection) -> Vec<i64> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT version FROM {LEDGER_TABLE} ORDER BY version"
        ))
        .expect("prepare");
    statement
        .query_map([], |row| row.get(0))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows")
}

#[test]
fn a_ledger_at_an_older_version_migrates_forward_without_data_loss() {
    let mut connection = Connection::open_in_memory().expect("database");
    assert_eq!(
        run_migrations(&mut connection, LEDGER_TABLE, &SCRIPTS[..1], &[]).expect("v1"),
        1
    );
    connection
        .execute_batch(
            "INSERT INTO agent_machines (id, name) VALUES ('m1', 'host');
             INSERT INTO agent_agents (id, name, name_key, command, working_folder, machine_id, status, created_at)
             VALUES ('a1', 'Ana', 'ana', 'claude', '/work', 'm1', 'stopped', 1);",
        )
        .expect("data at version 1");

    let applied = run_migrations(&mut connection, LEDGER_TABLE, SCRIPTS, &[]).expect("forward");

    assert_eq!(applied, SCRIPTS.len() - 1);
    assert_eq!(ledger_versions(&connection), all_versions());
    let agent_name: String = connection
        .query_row("SELECT name FROM agent_agents WHERE id = 'a1'", [], |row| {
            row.get(0)
        })
        .expect("agent kept");
    assert_eq!(agent_name, "Ana");
    connection
        .execute(
            "INSERT INTO agent_settings (key, value) VALUES ('quiet_period_ms', '1')",
            [],
        )
        .expect("version 2 table exists");
}

#[test]
fn a_name_key_that_differs_only_by_case_is_refused() {
    let mut connection = Connection::open_in_memory().expect("database");
    run_migrations(&mut connection, LEDGER_TABLE, SCRIPTS, &[]).expect("current");
    connection
        .execute_batch(
            "INSERT INTO agent_machines (id, name) VALUES ('m1', 'host');
             INSERT INTO agent_projects (id, name, name_key, folder) VALUES ('p1', 'Web', 'web', '/w');
             INSERT INTO agent_agents (id, name, name_key, command, machine_id, status, created_at)
             VALUES ('a1', 'Ana', 'ana', 'claude', 'm1', 'stopped', 1);",
        )
        .expect("first rows");

    let project = connection.execute(
        "INSERT INTO agent_projects (id, name, name_key, folder) VALUES ('p2', 'WEB', 'WEB', '/w')",
        [],
    );
    let agent = connection.execute(
        "INSERT INTO agent_agents (id, name, name_key, command, machine_id, status, created_at)
         VALUES ('a2', 'ANA', 'ANA', 'claude', 'm1', 'stopped', 1)",
        [],
    );

    assert!(project.is_err(), "project key ignores case");
    assert!(agent.is_err(), "agent key ignores case");
}

#[test]
fn a_second_run_changes_nothing() {
    let mut connection = Connection::open_in_memory().expect("database");
    run_migrations(&mut connection, LEDGER_TABLE, SCRIPTS, &[]).expect("first run");
    let schema = schema_dump(&connection);
    let ledger: Vec<(i64, i64)> = connection
        .prepare(&format!("SELECT version, applied_at FROM {LEDGER_TABLE}"))
        .expect("prepare")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows");

    let applied = run_migrations(&mut connection, LEDGER_TABLE, SCRIPTS, &[]).expect("second run");

    assert_eq!(applied, 0);
    assert_eq!(schema_dump(&connection), schema);
    let ledger_after: Vec<(i64, i64)> = connection
        .prepare(&format!("SELECT version, applied_at FROM {LEDGER_TABLE}"))
        .expect("prepare")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows");
    assert_eq!(ledger_after, ledger);
}

#[test]
fn a_failing_script_rolls_back_and_keeps_the_previous_version() {
    let mut connection = Connection::open_in_memory().expect("database");
    let scripts = [
        "CREATE TABLE agent_a (id TEXT);",
        "CREATE TABLE agent_b (id TEXT); SELECT nonsense FROM nowhere;",
    ];

    let error = run_migrations(&mut connection, LEDGER_TABLE, &scripts, &[]).expect_err("fails");

    assert!(
        matches!(error, StartupError::Migration { version: 2, .. }),
        "{error}"
    );
    assert_eq!(ledger_versions(&connection), [1]);
    let has_table_b: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = 'agent_b'",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(has_table_b, 0);
}

#[test]
fn a_database_newer_than_the_server_is_refused() {
    let mut connection = Connection::open_in_memory().expect("database");
    run_migrations(&mut connection, LEDGER_TABLE, SCRIPTS, &[]).expect("current");

    let error =
        run_migrations(&mut connection, LEDGER_TABLE, &SCRIPTS[..1], &[]).expect_err("downgrade");

    assert!(
        matches!(
            error,
            StartupError::DatabaseTooNew {
                found,
                supported: 1,
                ..
            } if usize::try_from(found).is_ok_and(|found| found == SCRIPTS.len())
        ),
        "{error}"
    );
}

#[test]
fn each_module_keeps_its_own_ledger() {
    let mut connection = Connection::open_in_memory().expect("database");
    run_migrations(&mut connection, LEDGER_TABLE, SCRIPTS, &[]).expect("agent");

    run_migrations(
        &mut connection,
        "terminal_schema_version",
        &["CREATE TABLE terminal_x (id TEXT);"],
        &[],
    )
    .expect("terminal");

    assert_eq!(ledger_versions(&connection), all_versions());
}

#[test]
fn a_ledger_name_that_is_not_a_plain_table_name_is_refused() {
    let mut connection = Connection::open_in_memory().expect("database");

    let error = run_migrations(&mut connection, "x; DROP TABLE y", &[], &[]).expect_err("refused");

    assert!(
        matches!(error, StartupError::InvalidLedgerName(_)),
        "{error}"
    );
}

/// A database at version 3 with one machine, so agent rows can reference it.
fn version_three_with(project_keys: &[(&str, &str)], agent_keys: &[(&str, &str)]) -> Connection {
    let mut connection = Connection::open_in_memory().expect("database");
    run_migrations(&mut connection, LEDGER_TABLE, &SCRIPTS[..3], &[]).expect("version 3");
    connection
        .execute_batch("INSERT INTO agent_machines (id, name) VALUES ('m1', 'host')")
        .expect("machine");
    for (index, (name, name_key)) in project_keys.iter().enumerate() {
        let id = format!("p{}", index + 1);
        connection
            .execute(
                "INSERT INTO agent_projects (id, name, name_key, folder) VALUES (?1, ?2, ?3, '/w')",
                rusqlite::params![id, name, name_key],
            )
            .expect("project row");
    }
    for (index, (name, name_key)) in agent_keys.iter().enumerate() {
        let id = format!("a{}", index + 1);
        connection
            .execute(
                "INSERT INTO agent_agents (id, name, name_key, command, machine_id, status, created_at)
                 VALUES (?1, ?2, ?3, 'claude', 'm1', 'stopped', 1)",
                rusqlite::params![id, name, name_key],
            )
            .expect("agent row");
    }
    connection
}

#[test]
fn a_database_with_ascii_case_duplicate_name_keys_stops_before_version_four() {
    let mut connection = version_three_with(
        &[("Web", "web"), ("WEB", "WEB")],
        &[("Ana", "ana"), ("ANA", "ANA")],
    );

    let error = run_migrations(&mut connection, LEDGER_TABLE, SCRIPTS, PRE_MIGRATION_CHECKS)
        .expect_err("collisions");

    let text = error.to_string();
    for fragment in [
        "agent_schema_version",
        "4",
        "agent_projects",
        "p1",
        "Web",
        "p2",
        "WEB",
        "agent_agents",
        "a1",
        "Ana",
        "a2",
        "ANA",
        "rename or delete",
    ] {
        assert!(text.contains(fragment), "missing {fragment} in: {text}");
    }
    assert_eq!(ledger_versions(&connection), [1, 2, 3]);
    let rows: i64 = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM agent_projects) + (SELECT COUNT(*) FROM agent_agents)",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(rows, 4, "the failed start must keep every row");
}

#[test]
fn a_database_without_duplicate_name_keys_migrates_to_version_five() {
    let mut connection = version_three_with(&[("Web", "web")], &[("Ana", "ana")]);

    let applied = run_migrations(&mut connection, LEDGER_TABLE, SCRIPTS, PRE_MIGRATION_CHECKS)
        .expect("migrates");

    assert_eq!(applied, SCRIPTS.len() - 3);
    assert_eq!(ledger_versions(&connection), all_versions());
}

#[test]
fn name_keys_that_differ_outside_ascii_case_migrate_to_version_five() {
    let mut connection = version_three_with(
        &[("Ana", "äna"), ("ANA", "ÄNA")],
        &[("Äna", "äna"), ("ÄNA", "ÄNA")],
    );

    let applied = run_migrations(&mut connection, LEDGER_TABLE, SCRIPTS, PRE_MIGRATION_CHECKS)
        .expect("migrates");

    assert_eq!(applied, SCRIPTS.len() - 3);
    assert_eq!(ledger_versions(&connection), all_versions());
}

#[test]
fn an_office_at_agent_version_five_upgrades_with_every_agent_and_no_task() {
    let mut connection = Connection::open_in_memory().expect("database");
    run_migrations(&mut connection, LEDGER_TABLE, &SCRIPTS[..5], &[]).expect("version 5");
    connection
        .execute_batch(
            "INSERT INTO agent_machines (id, name) VALUES ('m1', 'host');
             INSERT INTO agent_agents (id, name, name_key, command, working_folder, machine_id, status, created_at)
             VALUES ('a1', 'Ana', 'ana', 'claude', '/work', 'm1', 'stopped', 1),
                    ('a2', 'Ben', 'ben', 'claude', '/work', 'm1', 'exited', 2);",
        )
        .expect("agents at version 5");

    let applied = run_migrations(&mut connection, LEDGER_TABLE, SCRIPTS, &[]).expect("upgrade");

    assert_eq!(applied, SCRIPTS.len() - 5);
    assert_eq!(ledger_versions(&connection), all_versions());
    let agents_without_task: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM agent_agents \
             WHERE task_text IS NULL AND task_set_at IS NULL AND task_source IS NULL",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(agents_without_task, 2);
}
