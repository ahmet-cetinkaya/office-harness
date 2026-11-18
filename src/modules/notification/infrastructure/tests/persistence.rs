// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

use std::path::Path;
use std::sync::{Arc, Mutex};

use kernel::{AgentId, Timestamp};
use notification_domain::{Notification, NotificationEvent, NotificationId};
use notification_infrastructure::persistence::{NotificationRepository, SCRIPTS};
use rusqlite::Connection;

const NOW: Timestamp = Timestamp::from_unix_millis(1_700_000_000_000);

/// Opens the database file like a server start does, with the script applied.
fn open_office(database: &Path) -> NotificationRepository {
    let connection = Connection::open(database).expect("open database");
    migrated(connection)
}

fn memory_office() -> NotificationRepository {
    migrated(Connection::open_in_memory().expect("open"))
}

fn migrated(connection: Connection) -> NotificationRepository {
    for script in SCRIPTS {
        connection.execute_batch(script).expect("migrate");
    }
    NotificationRepository::new(Arc::new(Mutex::new(connection)))
}

fn row(agent_id: AgentId, name: &str, event: NotificationEvent, created_at: i64) -> Notification {
    Notification::new(
        NotificationId::generate(),
        agent_id,
        name.to_owned(),
        event,
        Timestamp::from_unix_millis(created_at),
    )
}

#[test]
fn a_stored_row_reads_back_with_its_copied_name_and_read_time() {
    let repository = memory_office();
    let stored = row(AgentId::generate(), "Ana", NotificationEvent::Exited, 10);
    repository.insert(&stored).expect("insert");

    let listed = repository.list().expect("list");

    assert_eq!(listed, vec![stored]);
}

#[test]
fn the_listing_is_unread_first_then_newest_first() {
    let repository = memory_office();
    let agent = AgentId::generate();
    let mut read_row = row(agent, "Ana", NotificationEvent::Bell, 30);
    read_row.mark_read(Timestamp::from_unix_millis(40));
    repository
        .insert(&row(agent, "Ana", NotificationEvent::Exited, 10))
        .expect("insert");
    repository
        .insert(&row(agent, "Ana", NotificationEvent::Failed, 20))
        .expect("insert");
    repository.insert(&read_row).expect("insert");

    let listed = repository.list().expect("list");

    let times: Vec<i64> = listed
        .iter()
        .map(|notification| notification.created_at.as_unix_millis())
        .collect();
    assert_eq!(times, [20, 10, 30]);
}

#[test]
fn unread_rows_of_one_agent_come_alone_and_oldest_first() {
    let repository = memory_office();
    let ana = AgentId::generate();
    let raj = AgentId::generate();
    let mut read_row = row(ana, "Ana", NotificationEvent::Exited, 5);
    read_row.mark_read(NOW);
    repository.insert(&read_row).expect("insert");
    repository
        .insert(&row(ana, "Ana", NotificationEvent::Bell, 20))
        .expect("insert");
    repository
        .insert(&row(ana, "Ana", NotificationEvent::Failed, 10))
        .expect("insert");
    repository
        .insert(&row(raj, "Raj", NotificationEvent::Exited, 30))
        .expect("insert");

    let listed = repository
        .list_unread_for_agent(ana)
        .expect("unread of ana");

    let times: Vec<i64> = listed
        .iter()
        .map(|notification| notification.created_at.as_unix_millis())
        .collect();
    assert_eq!(times, [10, 20]);
}

#[test]
fn an_update_persists_the_read_time() {
    let database = tempfile::tempdir().expect("folder");
    let repository = open_office(&database.path().join("office.db"));
    let stored = row(AgentId::generate(), "Ana", NotificationEvent::Bell, 10);
    repository.insert(&stored).expect("insert");
    let mut read_row = repository.find(stored.id).expect("find").expect("stored");
    read_row.mark_read(NOW);

    repository.update(&read_row).expect("update");

    let reread = repository.find(stored.id).expect("find").expect("stored");
    assert_eq!(reread.read_at, Some(NOW));
}

#[test]
fn a_row_stays_listed_when_no_agent_row_exists_anywhere() {
    let repository = memory_office();
    // No agent module table exists in this database at all: the row carries
    // the agent name itself, so it lists regardless.
    let stored = row(
        AgentId::generate(),
        "Deleted Ana",
        NotificationEvent::Exited,
        10,
    );
    repository.insert(&stored).expect("insert");

    let listed = repository.list().expect("list");

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].agent_name, "Deleted Ana");
}
