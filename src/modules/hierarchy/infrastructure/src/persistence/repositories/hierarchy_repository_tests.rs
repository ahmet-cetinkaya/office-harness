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

/// A failure on the lead's place write, the last of the three writes, rolls
/// back the team row and the membership row written before it.
#[test]
fn a_failed_team_save_leaves_no_team_and_no_membership() {
    let shared = database();
    let lead = AgentId::generate();
    {
        let connection = shared.lock().expect("database");
        agent(&connection, &lead, "Bo");
        connection
            .execute_batch(
                "CREATE TEMP TRIGGER refuse_place BEFORE UPDATE OF role ON agent_agents \
                 BEGIN SELECT RAISE(ABORT, 'place write refused'); END;",
            )
            .expect("trigger");
    }
    let store = repository(&shared);
    let team = hierarchy_domain::Team::new(
        hierarchy_domain::TeamName::parse("Core").expect("name"),
        None,
        lead,
    );

    let error = store
        .save_team_and_place_lead(&team, None)
        .expect_err("the place write fails");

    let HierarchyApplicationError::Storage(message) = &error else {
        panic!("the refusal reaches the store as Storage: {error}");
    };
    assert!(message.contains("place write refused"), "{message}");
    let connection = shared.lock().expect("database");
    let count = |table: &str| -> i64 {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("count")
    };
    assert_eq!(count("hierarchy_teams"), 0);
    assert_eq!(count("hierarchy_team_members"), 0);
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

/// A caller that holds the tree exclusion writes the team through the held
/// variants; a second take of the same exclusion would never return, so the
/// finished test is the proof.
#[test]
fn the_held_variants_write_and_delete_a_team_under_the_callers_exclusion() {
    let shared = database();
    let lead = AgentId::generate();
    {
        let connection = shared.lock().expect("database");
        agent(&connection, &lead, "Bo");
    }
    let store = repository(&shared);
    let team = hierarchy_domain::Team::new(
        hierarchy_domain::TeamName::parse("Core").expect("name"),
        None,
        lead,
    );

    let held = store.tree_exclusion();
    store
        .save_team_and_place_lead_held(&held, &team, None)
        .expect("saved");
    let teams_after_save = store.snapshot().expect("snapshot").teams.len();
    store
        .delete_team_and_unplace_held(&held, team.id())
        .expect("deleted");
    drop(held);

    assert_eq!(teams_after_save, 1);
    let after = store.snapshot().expect("snapshot");
    assert_eq!(after.teams.len(), 0);
    assert_eq!(after.rows.len(), 0, "the lead left the hierarchy");
}

/// The existing methods still take the exclusion themselves, so their callers
/// do not change.
#[test]
fn the_plain_team_writes_take_the_exclusion_themselves() {
    let shared = database();
    let lead = AgentId::generate();
    {
        let connection = shared.lock().expect("database");
        agent(&connection, &lead, "Bo");
    }
    let store = repository(&shared);
    let team = hierarchy_domain::Team::new(
        hierarchy_domain::TeamName::parse("Core").expect("name"),
        None,
        lead,
    );

    store.save_team_and_place_lead(&team, None).expect("saved");
    store.delete_team_and_unplace(team.id()).expect("deleted");

    assert_eq!(store.snapshot().expect("snapshot").teams.len(), 0);
}

fn parent_of(connection: &Connection, id: &AgentId) -> Option<String> {
    connection
        .query_row(
            "SELECT parent_agent_id FROM agent_agents WHERE id = ?1",
            params![id.as_uuid_text()],
            |row| row.get(0),
        )
        .expect("the agent row")
}

/// 0002 A5: teams can exist before the CEO; when the CEO is placed, the
/// leads that have no parent move under it, and a member keeps its lead.
#[test]
fn placing_a_ceo_moves_the_parentless_leads_under_it() {
    let shared = database();
    let (lead, member, ceo) = (
        AgentId::generate(),
        AgentId::generate(),
        AgentId::generate(),
    );
    let team_id = TeamId::generate();
    {
        let connection = shared.lock().expect("database");
        agent(&connection, &lead, "Lea");
        agent(&connection, &member, "Mo");
        agent(&connection, &ceo, "Cy");
        team(&connection, &team_id, &lead);
    }
    let store = repository(&shared);
    store
        .place_agent(lead, Some(Role::Lead), None, Some(team_id))
        .expect("the lead");
    store
        .place_agent(member, Some(Role::Member), Some(lead), Some(team_id))
        .expect("the member");

    store
        .place_agent(ceo, Some(Role::Ceo), None, None)
        .expect("the CEO");

    let connection = shared.lock().expect("database");
    assert_eq!(parent_of(&connection, &lead), Some(ceo.as_uuid_text()));
    assert_eq!(parent_of(&connection, &member), Some(lead.as_uuid_text()));
    assert_eq!(parent_of(&connection, &ceo), None);
}

/// The adoption is part of the CEO's placement: when the placement fails,
/// no lead moves.
#[test]
fn a_failed_ceo_placement_moves_no_lead() {
    let shared = database();
    let (lead, ceo) = (AgentId::generate(), AgentId::generate());
    let team_id = TeamId::generate();
    {
        let connection = shared.lock().expect("database");
        agent(&connection, &lead, "Lea");
        agent(&connection, &ceo, "Cy");
        team(&connection, &team_id, &lead);
    }
    let store = repository(&shared);
    store
        .place_agent(lead, Some(Role::Lead), None, Some(team_id))
        .expect("the lead");
    {
        let connection = shared.lock().expect("database");
        // The failure lands on the adoption UPDATE, after the CEO's own
        // place write in the same transaction.
        connection
            .execute_batch(
                "CREATE TEMP TRIGGER refuse_adoption BEFORE UPDATE OF parent_agent_id \
                 ON agent_agents WHEN OLD.role = 'lead' \
                 BEGIN SELECT RAISE(ABORT, 'adoption refused'); END;",
            )
            .expect("trigger");
    }

    let error = store
        .place_agent(ceo, Some(Role::Ceo), None, None)
        .expect_err("the adoption fails");

    assert!(error.to_string().contains("adoption refused"), "{error}");
    let connection = shared.lock().expect("database");
    assert_eq!(parent_of(&connection, &lead), None);
    let ceo_role: Option<String> = connection
        .query_row(
            "SELECT role FROM agent_agents WHERE id = ?1",
            params![ceo.as_uuid_text()],
            |row| row.get(0),
        )
        .expect("the CEO row");
    assert_eq!(ceo_role, None);
}
