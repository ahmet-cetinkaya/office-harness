// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

use std::path::{Path, PathBuf};

use agent_domain::entities::{Agent, Machine, Project};
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{AgentCommand, DisplayName, ProjectId, SettingKey, Settings};
use agent_infrastructure::persistence::{
    AgentRepository, MachineRepository, PersistenceError, ProjectRepository, SCRIPTS,
    SettingsRepository, SharedConnection,
};
use kernel::{AgentId, Timestamp};
use rusqlite::Connection;
use tempfile::TempDir;

struct Office {
    machine: Machine,
    agents: AgentRepository,
    projects: ProjectRepository,
    settings: SettingsRepository,
}

/// Opens the database file like a server start does, with every script applied
/// once.
fn open_office(database: &Path) -> Office {
    let connection = Connection::open(database).expect("open database");
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .expect("pragma");
    let is_new = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = 'agent_agents'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("count")
        == 0;
    if is_new {
        for script in SCRIPTS {
            connection.execute_batch(script).expect("migrate");
        }
    }
    let connection = SharedConnection::new(connection);
    Office {
        machine: MachineRepository::new(connection.clone())
            .ensure_local("office-host")
            .expect("machine"),
        agents: AgentRepository::new(connection.clone()),
        projects: ProjectRepository::new(connection.clone()),
        settings: SettingsRepository::new(connection),
    }
}

fn database_in(folder: &TempDir) -> PathBuf {
    folder.path().join("office.db")
}

fn agent_named(office: &Office, name: &str, project_id: Option<ProjectId>) -> Agent {
    Agent::new(
        AgentId::generate(),
        DisplayName::parse("name", name).expect("name"),
        AgentCommand::parse("claude --verbose").expect("command"),
        Some(std::env::temp_dir()),
        project_id,
        office.machine.id,
        Timestamp::from_unix_millis(1_700_000_000_000),
    )
    .expect("agent")
}

fn project_named(name: &str) -> Project {
    Project::new(
        ProjectId::generate(),
        DisplayName::parse("name", name).expect("name"),
        std::env::temp_dir(),
    )
    .expect("project")
}

#[test]
fn definitions_and_settings_survive_a_restart() {
    let folder = tempfile::tempdir().expect("folder");
    let database = database_in(&folder);
    let first = open_office(&database);
    let project = project_named("Website");
    first.projects.insert(&project).expect("project");
    let agent = agent_named(&first, "Ana", Some(project.id));
    first.agents.insert(&agent).expect("agent");
    let mut settings = Settings::default();
    settings
        .set_from_text(SettingKey::QuietPeriodMs, "1234")
        .expect("setting");
    first.settings.save(&settings).expect("save settings");
    let machine = first.machine.clone();
    drop(first);

    let second = open_office(&database);

    assert_eq!(second.machine, machine);
    assert_eq!(second.agents.list().expect("agents"), [agent]);
    assert_eq!(second.projects.list().expect("projects"), [project]);
    assert_eq!(second.settings.load().expect("settings"), settings);
}

#[test]
fn settings_default_until_saved() {
    let folder = tempfile::tempdir().expect("folder");
    let office = open_office(&database_in(&folder));

    assert_eq!(
        office.settings.load().expect("settings"),
        Settings::default()
    );
}

#[test]
fn only_running_agents_become_interrupted_and_none_starts() {
    let folder = tempfile::tempdir().expect("folder");
    let office = open_office(&database_in(&folder));
    let statuses = AgentStatus::ALL;
    let agents: Vec<Agent> = statuses
        .iter()
        .map(|status| agent_named(&office, &format!("agent {status}"), None))
        .collect();
    for (agent, status) in agents.iter().zip(statuses) {
        office.agents.insert(agent).expect("insert");
        office
            .agents
            .update_status(agent.id, status)
            .expect("status");
    }

    let changed = office.agents.mark_running_as_interrupted().expect("sweep");

    assert_eq!(changed, AgentStatus::RUNNING.len());
    for (agent, before) in agents.iter().zip(statuses) {
        let after = office
            .agents
            .find(agent.id)
            .expect("find")
            .expect("exists")
            .status;
        let expected = if before.is_running() {
            AgentStatus::Interrupted
        } else {
            before
        };
        assert_eq!(after, expected, "was {before}");
    }
    assert_eq!(
        office.agents.mark_running_as_interrupted().expect("again"),
        0
    );
}

#[test]
fn agent_and_project_names_are_unique_ignoring_case() {
    let folder = tempfile::tempdir().expect("folder");
    let office = open_office(&database_in(&folder));
    office
        .agents
        .insert(&agent_named(&office, "Ünal", None))
        .expect("first");
    office
        .projects
        .insert(&project_named("Site"))
        .expect("first");

    let agent_error = office
        .agents
        .insert(&agent_named(&office, "ünal", None))
        .expect_err("duplicate");
    let project_error = office
        .projects
        .insert(&project_named("SITE"))
        .expect_err("duplicate");

    assert!(
        matches!(agent_error, PersistenceError::Duplicate { .. }),
        "{agent_error}"
    );
    assert!(
        matches!(project_error, PersistenceError::Duplicate { .. }),
        "{project_error}"
    );
}

#[test]
fn deleting_a_project_in_use_is_refused_and_names_the_agents() {
    let folder = tempfile::tempdir().expect("folder");
    let office = open_office(&database_in(&folder));
    let project = project_named("Website");
    office.projects.insert(&project).expect("project");
    let agent = agent_named(&office, "Ana", Some(project.id));
    office.agents.insert(&agent).expect("agent");

    let error = office.projects.delete(project.id).expect_err("in use");

    assert_eq!(
        error.to_string(),
        "project Website is still used by the agents Ana"
    );
    office.agents.delete(agent.id).expect("delete agent");
    office.projects.delete(project.id).expect("delete project");
    assert!(office.projects.find(project.id).expect("find").is_none());
}

#[test]
fn the_first_start_creates_the_one_machine_and_later_starts_reuse_it() {
    let folder = tempfile::tempdir().expect("folder");
    let database = database_in(&folder);
    let first = open_office(&database).machine;

    let second = open_office(&database).machine;

    assert_eq!(first, second);
    assert_eq!(first.name, "office-host");
}
