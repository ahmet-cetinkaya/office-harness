// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::models::SweepOutcome;
use agent_infrastructure::persistence::{LEDGER_TABLE, PRE_MIGRATION_CHECKS, SCRIPTS};
use audit_domain::{ActionKind, ActionResult};
use audit_infrastructure::persistence::{
    LEDGER_TABLE as AUDIT_LEDGER_TABLE, SCRIPTS as AUDIT_SCRIPTS,
};
use kernel::{Actor, Timestamp};

use super::*;
use crate::bootstrap::run_migrations;

const IDENTITY_ID: i64 = 7;

fn office_database() -> Arc<Mutex<Connection>> {
    let mut connection = Connection::open_in_memory().expect("database");
    run_migrations(&mut connection, LEDGER_TABLE, SCRIPTS, PRE_MIGRATION_CHECKS).expect("agent");
    run_migrations(&mut connection, AUDIT_LEDGER_TABLE, AUDIT_SCRIPTS, &[]).expect("audit");
    // The row stands alone: the test needs no agent and session behind it.
    connection
        .execute_batch("PRAGMA foreign_keys = OFF")
        .expect("pragma");
    connection
        .execute(
            "INSERT INTO agent_session_processes \
             (id, session_id, pid, started_at, command_hash, command_display, boot, kind, state) \
             VALUES (?1, 'session', 10, 20, x'00', 'sleep', 'boot', 'cli', 'open')",
            [IDENTITY_ID],
        )
        .expect("identity row");
    Arc::new(Mutex::new(connection))
}

fn record() -> ActionRecord {
    ActionRecord {
        actor: Actor::Office,
        target_agent_id: None,
        action: ActionKind::EndLeftover,
        result: ActionResult::Ok,
        detail: Some("identity 7: ended".to_owned()),
        at: Timestamp::from_unix_millis(1),
    }
}

fn identity_state(database: &Mutex<Connection>) -> String {
    database
        .lock()
        .expect("lock")
        .query_row(
            "SELECT state FROM agent_session_processes WHERE id = ?1",
            [IDENTITY_ID],
            |row| row.get(0),
        )
        .expect("state")
}

fn entry_count(database: &Mutex<Connection>) -> i64 {
    database
        .lock()
        .expect("lock")
        .query_row("SELECT COUNT(*) FROM audit_actions", [], |row| row.get(0))
        .expect("count")
}

#[test]
fn the_entry_and_the_ended_mark_are_written_together() {
    let database = office_database();

    let entry = SharedLedger::new(Arc::clone(&database))
        .record(IDENTITY_ID, SweepOutcome::Ended, &record())
        .expect("recorded");

    assert_eq!(entry.record.action, ActionKind::EndLeftover);
    assert_eq!(entry_count(&database), 1);
    assert_eq!(identity_state(&database), "ended");
    let outcome: String = database
        .lock()
        .expect("lock")
        .query_row(
            "SELECT sweep_outcome FROM agent_session_processes WHERE id = ?1",
            [IDENTITY_ID],
            |row| row.get(0),
        )
        .expect("outcome");
    assert_eq!(outcome, "ended");
}

#[test]
fn a_failed_entry_leaves_the_identity_open_so_the_next_sweep_tries_again() {
    let database = office_database();
    database
        .lock()
        .expect("lock")
        .execute("DROP TABLE audit_actions", [])
        .expect("drop");

    let outcome = SharedLedger::new(Arc::clone(&database)).record(
        IDENTITY_ID,
        SweepOutcome::Ended,
        &record(),
    );

    assert!(outcome.is_err());
    assert_eq!(identity_state(&database), "open");
}

#[test]
fn a_failed_mark_leaves_no_entry_behind() {
    let database = office_database();
    database
        .lock()
        .expect("lock")
        .execute_batch(
            "CREATE TRIGGER refuse_end BEFORE UPDATE ON agent_session_processes \
             BEGIN SELECT RAISE(ABORT, 'refused'); END;",
        )
        .expect("trigger");

    let outcome = SharedLedger::new(Arc::clone(&database)).record(
        IDENTITY_ID,
        SweepOutcome::Ended,
        &record(),
    );

    assert!(outcome.is_err());
    assert_eq!(entry_count(&database), 0);
    assert_eq!(identity_state(&database), "open");
}
