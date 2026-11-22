// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

use std::path::{Path, PathBuf};

use agent_domain::entities::{AdditionsRecord, Agent, Machine, Project, Session};
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{
    AgentCommand, DisplayName, EnvEntry, ExitResult, OfficeAdditions, ProjectId, SessionId,
    SettingKey, Settings, TaskSource, TaskText,
};
use agent_infrastructure::persistence::{
    AdditionsRepository, AgentRepository, MachineRepository, PersistenceError, ProjectRepository,
    SCRIPTS, SessionRepository, SettingsRepository, SharedConnection,
};
use kernel::{AgentId, Timestamp};
use rusqlite::Connection;
use tempfile::TempDir;

struct Office {
    machine: Machine,
    agents: AgentRepository,
    projects: ProjectRepository,
    settings: SettingsRepository,
    sessions: SessionRepository,
    additions: AdditionsRepository,
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
        settings: SettingsRepository::new(connection.clone()),
        sessions: SessionRepository::new(connection.clone()),
        additions: AdditionsRepository::new(connection),
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
fn a_stored_focus_shortcut_that_became_reserved_falls_back_to_the_default() {
    let folder = tempfile::tempdir().expect("folder");
    let database = database_in(&folder);
    drop(open_office(&database));
    // Valid before 0007 reserved Ctrl+Shift+H for the office hold.
    Connection::open(&database)
        .expect("open database")
        .execute(
            "INSERT INTO agent_settings (key, value) VALUES ('focus_shortcut', 'Ctrl+Shift+H')",
            [],
        )
        .expect("store the old shortcut");

    let loaded = open_office(&database)
        .settings
        .load()
        .expect("settings load");

    assert_eq!(loaded.focus_shortcut, Settings::default().focus_shortcut);
}

#[test]
fn a_stored_focus_shortcut_loads_kept_or_falls_back_by_the_reserved_table() {
    let jump_key_spellings = ["Ctrl+Shift+F11", "Shift+Ctrl+F11"];
    let reserved_keys = ["Ctrl+Shift+F1", "Ctrl+Shift+F10", "Ctrl+Shift+H"];
    let free_keys = ["Ctrl+Alt+F11", "Ctrl+Shift+F12"];
    let default = Settings::default().focus_shortcut;
    let cases = jump_key_spellings
        .iter()
        .chain(&free_keys)
        .map(|stored| (*stored, *stored))
        .chain(
            reserved_keys
                .iter()
                .map(|stored| (*stored, default.as_str())),
        );
    for (stored, expected) in cases {
        let folder = tempfile::tempdir().expect("folder");
        let database = database_in(&folder);
        drop(open_office(&database));
        Connection::open(&database)
            .expect("open database")
            .execute(
                "INSERT INTO agent_settings (key, value) VALUES ('focus_shortcut', ?1)",
                [stored],
            )
            .expect("store the shortcut");

        let loaded = open_office(&database)
            .settings
            .load()
            .expect("settings load");

        assert_eq!(loaded.focus_shortcut, expected, "{stored}");
    }
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

fn additions_of(flag: &str, prompt: &str) -> OfficeAdditions {
    OfficeAdditions::new(
        vec![flag.to_owned()],
        vec![EnvEntry::parse("KEY", "value").expect("entry")],
        Some(prompt.to_owned()),
    )
    .expect("additions")
}

fn session_of(agent: &Agent, millis: i64, additions: OfficeAdditions) -> Session {
    Session::begin(
        SessionId::generate(),
        agent.id,
        Some("cli-1".to_owned()),
        None,
        additions,
        Timestamp::from_unix_millis(millis),
    )
}

#[test]
fn an_agents_additions_are_replaced_and_survive_a_reopen() {
    let folder = TempDir::new().expect("folder");
    let first = open_office(&database_in(&folder));
    let agent = agent_named(&first, "Ana", None);
    first.agents.insert(&agent).expect("agent");
    let record = |flag, millis| AdditionsRecord {
        additions: additions_of(flag, "hello"),
        updated_at: Timestamp::from_unix_millis(millis),
    };
    assert_eq!(first.additions.load(agent.id).expect("load"), None);

    first
        .additions
        .save(agent.id, &record("--one", 1))
        .expect("save");
    first
        .additions
        .save(agent.id, &record("--two", 2))
        .expect("replace");
    drop(first);

    let reopened = open_office(&database_in(&folder));
    assert_eq!(
        reopened.additions.load(agent.id).expect("load"),
        Some(record("--two", 2))
    );
}

#[test]
fn sessions_keep_their_additions_snapshot_and_list_newest_first() {
    let folder = TempDir::new().expect("folder");
    let office = open_office(&database_in(&folder));
    let agent = agent_named(&office, "Ana", None);
    office.agents.insert(&agent).expect("agent");
    let older = session_of(&agent, 10, additions_of("--old", "p1"));
    let newer = session_of(&agent, 20, additions_of("--new", "p2"));
    office.sessions.insert(&older).expect("older");
    office.sessions.insert(&newer).expect("newer");

    let listed = office.sessions.list_for_agent(agent.id).expect("list");

    assert_eq!(listed, [newer.clone(), older.clone()]);
    assert_eq!(listed[1].additions_snapshot.flags(), ["--old"]);
    assert_eq!(
        office.sessions.latest_for_agent(agent.id).expect("latest"),
        Some(newer)
    );
    assert_eq!(office.sessions.find(older.id).expect("find"), Some(older));
}

#[test]
fn ending_the_running_session_happens_once_and_keeps_the_first_result() {
    let folder = TempDir::new().expect("folder");
    let office = open_office(&database_in(&folder));
    let agent = agent_named(&office, "Ana", None);
    office.agents.insert(&agent).expect("agent");
    let session = session_of(&agent, 10, OfficeAdditions::default());
    office.sessions.insert(&session).expect("session");
    let at = Timestamp::from_unix_millis(50);

    let first = office
        .sessions
        .end_running(agent.id, &ExitResult::Code(2), at)
        .expect("end");
    let second = office
        .sessions
        .end_running(agent.id, &ExitResult::Signal, at)
        .expect("again");

    assert!(first);
    assert!(!second);
    let stored = office
        .sessions
        .find(session.id)
        .expect("find")
        .expect("stored");
    assert_eq!(stored.exit_result, Some(ExitResult::Code(2)));
    assert_eq!(stored.ended_at, Some(at));
}

#[test]
fn a_cli_session_id_is_set_later_and_an_unknown_session_is_not_found() {
    let folder = TempDir::new().expect("folder");
    let office = open_office(&database_in(&folder));
    let agent = agent_named(&office, "Ana", None);
    office.agents.insert(&agent).expect("agent");
    let mut session = session_of(&agent, 10, OfficeAdditions::default());
    session.cli_session_id = None;
    office.sessions.insert(&session).expect("session");

    office
        .sessions
        .set_cli_session_id(session.id, "found")
        .expect("set");

    let stored = office
        .sessions
        .find(session.id)
        .expect("find")
        .expect("stored");
    assert_eq!(stored.cli_session_id.as_deref(), Some("found"));
    assert!(matches!(
        office
            .sessions
            .set_cli_session_id(SessionId::generate(), "x"),
        Err(PersistenceError::NotFound { .. })
    ));
}

#[test]
fn sessions_still_running_at_the_next_start_become_interrupted() {
    let folder = TempDir::new().expect("folder");
    let office = open_office(&database_in(&folder));
    let agent = agent_named(&office, "Ana", None);
    office.agents.insert(&agent).expect("agent");
    let session = session_of(&agent, 10, OfficeAdditions::default());
    office.sessions.insert(&session).expect("session");

    let changed = office
        .sessions
        .mark_running_as_interrupted(Timestamp::from_unix_millis(99))
        .expect("interrupt");

    assert_eq!(changed, 1);
    let stored = office
        .sessions
        .find(session.id)
        .expect("find")
        .expect("stored");
    assert_eq!(stored.exit_result, Some(ExitResult::Interrupted));
}

#[test]
fn deleting_an_agent_deletes_its_additions_and_sessions() {
    let folder = TempDir::new().expect("folder");
    let office = open_office(&database_in(&folder));
    let agent = agent_named(&office, "Ana", None);
    office.agents.insert(&agent).expect("agent");
    let session = session_of(&agent, 10, OfficeAdditions::default());
    office.sessions.insert(&session).expect("session");
    office
        .additions
        .save(
            agent.id,
            &AdditionsRecord {
                additions: additions_of("--x", "p"),
                updated_at: Timestamp::from_unix_millis(1),
            },
        )
        .expect("additions");

    office.agents.delete(agent.id).expect("delete");

    assert_eq!(office.sessions.find(session.id).expect("find"), None);
    assert_eq!(office.additions.load(agent.id).expect("load"), None);
}

#[test]
fn a_task_text_survives_a_reopen_and_a_clear_removes_it() {
    let folder = TempDir::new().expect("folder");
    let first = open_office(&database_in(&folder));
    let agent = agent_named(&first, "Ana", None);
    first.agents.insert(&agent).expect("agent");
    let control_text = "Fix \u{1b}[31mit\nthen report";
    let task = TaskText::from_input(
        control_text,
        Timestamp::from_unix_millis(42),
        TaskSource::Directive,
    )
    .expect("valid")
    .expect("some");
    first
        .agents
        .update_task(agent.id, Some(&task))
        .expect("set");
    drop(first);

    let reopened = open_office(&database_in(&folder));
    let stored = reopened
        .agents
        .find(agent.id)
        .expect("find")
        .expect("agent");
    assert_eq!(stored.task, Some(task));
    reopened.agents.update_task(agent.id, None).expect("clear");
    let cleared = reopened
        .agents
        .find(agent.id)
        .expect("find")
        .expect("agent");
    assert_eq!(cleared.task, None);
}

#[test]
fn a_task_text_of_an_unknown_agent_is_not_found() {
    let folder = TempDir::new().expect("folder");
    let office = open_office(&database_in(&folder));

    let error = office
        .agents
        .update_task(AgentId::generate(), None)
        .expect_err("unknown");

    assert!(
        matches!(error, PersistenceError::NotFound { .. }),
        "{error}"
    );
}

#[test]
fn a_stored_task_the_domain_rejects_reads_as_no_task_and_keeps_the_agent_readable() {
    let folder = TempDir::new().expect("folder");
    let database = database_in(&folder);
    let office = open_office(&database);
    let broken = agent_named(&office, "Ana", None);
    let healthy = agent_named(&office, "Bo", None);
    office.agents.insert(&broken).expect("agent");
    office.agents.insert(&healthy).expect("agent");
    Connection::open(&database)
        .expect("open database")
        .execute(
            "UPDATE agent_agents SET task_text = 'x', task_set_at = 1, \
             task_source = 'from a later version' WHERE id = ?1",
            [broken.id.to_string()],
        )
        .expect("hand edit");

    let listed = office.agents.list().expect("the list is still readable");

    assert_eq!(listed.len(), 2);
    assert!(listed.iter().all(|agent| agent.task.is_none()));
}
