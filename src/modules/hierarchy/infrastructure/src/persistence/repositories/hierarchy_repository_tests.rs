// SPDX-License-Identifier: AGPL-3.0-or-later

//! Repository tests against an in-memory database. The fixture runs the
//! real migration SQL (this crate's own plus the agent module's, included
//! verbatim), so the schema the tests see is the schema the server runs.

use std::sync::{Arc, Mutex};

use hierarchy_application::common::errors::HierarchyApplicationError;
use hierarchy_application::common::interfaces::HierarchyStore;
use hierarchy_domain::entities::TeamId;
use hierarchy_domain::value_objects::Role;
use kernel::AgentId;
use rusqlite::{Connection, params};

use super::hierarchy_repository::HierarchyRepository;
use crate::persistence::migrations::SCRIPTS;
use crate::services::agent_places::AgentPlaces;

/// The team table the migration owns, plus the agent module's own
/// `agent_agents` migrations run verbatim, so the fixture cannot drift
/// from the real schema: the SQL is included from the agent crate's files
/// at compile time, not copied by hand.
fn schema() -> String {
    let mut script = SCRIPTS.concat();
    for migration in [
        include_str!(
            "../../../../../agent/infrastructure/src/persistence/migrations/0001_machines_projects_agents.sql"
        ),
        include_str!(
            "../../../../../agent/infrastructure/src/persistence/migrations/0002_settings.sql"
        ),
        include_str!(
            "../../../../../agent/infrastructure/src/persistence/migrations/0003_additions_and_sessions.sql"
        ),
        include_str!(
            "../../../../../agent/infrastructure/src/persistence/migrations/0004_name_key_nocase.sql"
        ),
        include_str!(
            "../../../../../agent/infrastructure/src/persistence/migrations/0005_hierarchy_columns.sql"
        ),
    ] {
        script.push_str(migration);
    }
    script
}

/// The shared database behind the repository, so a test can read state
/// through the same lock the store uses.
fn database() -> Arc<Mutex<Connection>> {
    let connection = Connection::open_in_memory().expect("database");
    connection.execute_batch(&schema()).expect("schema");
    Arc::new(Mutex::new(connection))
}

fn agent(connection: &Connection, id: &AgentId, name: &str) {
    connection
        .execute(
            "INSERT OR IGNORE INTO agent_machines (id, name) VALUES ('machine', 'The Machine')",
            [],
        )
        .expect("machine row");
    connection
        .execute(
            "INSERT INTO agent_agents (id, name, name_key, command, machine_id, status, created_at) \
             VALUES (?1, ?2, ?3, 'cli', 'machine', 'stopped', 0)",
            params![id.as_uuid_text(), name, name.to_lowercase()],
        )
        .expect("agent row");
}

fn team(connection: &Connection, id: &TeamId, lead: &AgentId) {
    connection
        .execute(
            "INSERT INTO hierarchy_teams (id, name, name_key, lead_agent_id) \
             VALUES (?1, 'The Team', 'the team', ?2)",
            params![id.as_uuid_text(), lead.as_uuid_text()],
        )
        .expect("team row");
}

fn repository(shared: &Arc<Mutex<Connection>>) -> HierarchyRepository {
    HierarchyRepository::new(Arc::clone(shared), AgentPlaces::new())
}

/// The place columns, the membership row, and the team's seat together or
/// not at all: a failure between the writes leaves no partial placement.
#[test]
fn a_failed_lead_placement_writes_nothing() {
    let shared = database();
    let seated = AgentId::generate();
    let lead = AgentId::generate();
    let team_id = TeamId::generate();
    {
        let connection = shared.lock().expect("database");
        agent(&connection, &seated, "Sid");
        agent(&connection, &lead, "Bo");
        team(&connection, &team_id, &seated);
        // The failure lands on the lead-seat UPDATE, after the place
        // columns and the membership row are already written: a trigger
        // aborts exactly that statement.
        connection
            .execute_batch(
                "CREATE TEMP TRIGGER refuse_seat_change BEFORE UPDATE OF lead_agent_id \
                 ON hierarchy_teams BEGIN SELECT RAISE(ABORT, 'seat write refused'); END;",
            )
            .expect("trigger");
    }

    let placed = repository(&shared).place_agent(lead, Some(Role::Lead), None, Some(team_id));
    // The failure is the seat UPDATE's own refusal, not an earlier write:
    // the test proves a mid-transaction rollback, not just any error.
    let error = placed.expect_err("the seat UPDATE fails");
    let HierarchyApplicationError::Storage(message) = &error else {
        panic!("the seat refusal reaches the store as Storage: {error}");
    };
    assert!(message.contains("seat write refused"), "{message}");

    let connection = shared.lock().expect("database");
    // The place-column UPDATE and the membership INSERT ran before the
    // failing one; the transaction rolled both back, so no part of the
    // placement survived and the seated lead keeps the seat.
    let placed_row: Option<String> = connection
        .query_row(
            "SELECT role FROM agent_agents WHERE id = ?1",
            params![lead.as_uuid_text()],
            |row| row.get(0),
        )
        .expect("the agent row");
    let member: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM hierarchy_team_members WHERE agent_id = ?1",
            params![lead.as_uuid_text()],
            |row| row.get(0),
        )
        .expect("members");
    let seat: String = connection
        .query_row(
            "SELECT lead_agent_id FROM hierarchy_teams WHERE id = ?1",
            params![team_id.as_uuid_text()],
            |row| row.get(0),
        )
        .expect("the team row");
    assert_eq!(placed_row, None);
    assert_eq!(member, 0);
    assert_eq!(seat, seated.as_uuid_text());
}

/// The green path still seats the lead in the same transaction.
#[test]
fn a_lead_placement_seats_the_lead_in_the_same_write() {
    let shared = database();
    let seated = AgentId::generate();
    let lead = AgentId::generate();
    let team_id = TeamId::generate();
    {
        let connection = shared.lock().expect("database");
        agent(&connection, &seated, "Sid");
        agent(&connection, &lead, "Bo");
        team(&connection, &team_id, &seated);
    }

    repository(&shared)
        .place_agent(lead, Some(Role::Lead), None, Some(team_id))
        .expect("placement");

    let connection = shared.lock().expect("database");
    let seat: String = connection
        .query_row(
            "SELECT lead_agent_id FROM hierarchy_teams WHERE id = ?1",
            params![team_id.as_uuid_text()],
            |row| row.get(0),
        )
        .expect("the team row");
    let member: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM hierarchy_team_members WHERE agent_id = ?1",
            params![lead.as_uuid_text()],
            |row| row.get(0),
        )
        .expect("members");
    assert_eq!(seat, lead.as_uuid_text());
    assert_eq!(member, 1);
}
