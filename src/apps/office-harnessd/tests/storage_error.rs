// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

mod support;

use agent_domain::entities::Agent;
use agent_domain::value_objects::AgentCommand;
use agent_infrastructure::persistence::{
    AgentRepository, LEDGER_TABLE, MachineRepository, PersistenceError, SCRIPTS, SharedConnection,
};
use office_harnessd::bootstrap::{self, STORAGE_ERROR_EVENT, StorageAlerts};
use office_harnessd::config::Config;
use support::{agent_named, config_in};

const FILLER_BYTES: usize = 4_000;
const MAX_ATTEMPTS: usize = 2_000;

/// An agent whose command is large, so that each insert needs new pages.
fn bulky_agent(machine_id: agent_domain::value_objects::MachineId, number: usize) -> Agent {
    let mut agent = agent_named(machine_id, &format!("agent {number}"));
    agent.command = AgentCommand::parse(&"x".repeat(FILLER_BYTES)).expect("command");
    agent
}

/// Repositories on a database that is capped at its current size: the next
/// growth hits `SQLITE_FULL`, the error a full disk produces.
fn capped_repositories(config: &Config) -> (AgentRepository, MachineRepository) {
    let mut connection = bootstrap::open_database(&config.database_path()).expect("database");
    bootstrap::run_migrations(&mut connection, LEDGER_TABLE, SCRIPTS, &[]).expect("migrations");
    let page_count: i64 = connection
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .expect("pages");
    connection
        .execute_batch(&format!("PRAGMA max_page_count = {page_count}"))
        .expect("cap");
    let connection = SharedConnection::new(connection);
    (
        AgentRepository::new(connection.clone()),
        MachineRepository::new(connection),
    )
}

#[test]
fn a_full_disk_raises_storage_error_and_the_server_keeps_working() {
    let folder = tempfile::tempdir().expect("folder");
    let config = config_in(&folder);
    let log = bootstrap::open_log(&config).expect("log");
    let alerts = StorageAlerts::new(log);
    let mut subscriber = alerts.subscribe();
    let (agents, machines) = capped_repositories(&config);
    let machine_id = machines.ensure_local("host").expect("machine").id;

    let failure = (0..MAX_ATTEMPTS)
        .find_map(|number| {
            alerts
                .guard(agents.insert(&bulky_agent(machine_id, number)))
                .err()
        })
        .expect("the capped database must refuse a write");

    assert!(failure.is_storage_failure(), "{failure}");
    let alert = subscriber.try_recv().expect("storage.error event");
    assert_eq!(alert.message, failure.to_string());
    assert_eq!(STORAGE_ERROR_EVENT, "storage.error");
    assert!(agents.list().is_ok(), "reads still work");
    let log_text = std::fs::read_to_string(config.log_path()).expect("log");
    assert!(log_text.contains("storage.error: "), "{log_text}");
}

#[test]
fn a_rejected_request_is_not_a_storage_error() {
    let folder = tempfile::tempdir().expect("folder");
    let config = config_in(&folder);
    let log = bootstrap::open_log(&config).expect("log");
    let services = bootstrap::start(&config, &log).expect("start");
    let alerts = StorageAlerts::new(log);
    let mut subscriber = alerts.subscribe();
    let agent = agent_named(services.machine.id, "Ana");
    services.agents.insert(&agent).expect("first");

    let duplicate = alerts.guard(
        services
            .agents
            .insert(&agent_named(services.machine.id, "ana")),
    );

    assert!(matches!(duplicate, Err(PersistenceError::Duplicate { .. })));
    assert!(subscriber.try_recv().is_err());
}
