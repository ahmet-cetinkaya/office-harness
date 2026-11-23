// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

use std::path::{Path, PathBuf};

use agent_application::common::models::{ProcessKind, SessionProcess, SweepOutcome, SweepSummary};
use agent_domain::entities::{
    AdditionsRecord, Agent, FinishedReport, FinishedTemp, Machine, Project, Session,
};
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{
    AgentCommand, DisplayName, EnvEntry, ExitResult, FinishedTempId, Objective, OfficeAdditions,
    ProjectId, SessionId, SettingKey, Settings, TaskSource, TaskText, TempMark,
};
use agent_infrastructure::persistence::{
    AdditionsRepository, AgentRepository, FinishedTempRepository, MachineRepository,
    PersistenceError, ProjectRepository, SCRIPTS, SessionProcessRepository, SessionRepository,
    SettingsRepository, SharedConnection,
};
use kernel::{Actor, AgentId, Timestamp};
use rusqlite::Connection;
use tempfile::TempDir;

struct Office {
    machine: Machine,
    agents: AgentRepository,
    projects: ProjectRepository,
    settings: SettingsRepository,
    sessions: SessionRepository,
    processes: SessionProcessRepository,
    additions: AdditionsRepository,
    finished_temps: FinishedTempRepository,
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
        processes: SessionProcessRepository::new(connection.clone()),
        additions: AdditionsRepository::new(connection.clone()),
        finished_temps: FinishedTempRepository::new(connection),
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
        vec![EnvEntry::parse("KEY", Some("value")).expect("entry")],
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
fn sessions_still_open_at_the_next_start_become_lost() {
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
    assert_eq!(stored.exit_result, Some(ExitResult::Lost));
}

#[test]
fn a_clean_stop_ends_the_session_as_interrupted_and_leaves_nothing_for_the_next_start() {
    let folder = TempDir::new().expect("folder");
    let office = open_office(&database_in(&folder));
    let agent = agent_named(&office, "Ana", None);
    office.agents.insert(&agent).expect("agent");
    let session = session_of(&agent, 10, OfficeAdditions::default());
    office.sessions.insert(&session).expect("session");
    office
        .sessions
        .end_running(
            agent.id,
            &ExitResult::Interrupted,
            Timestamp::from_unix_millis(50),
        )
        .expect("clean stop");

    let changed = office
        .sessions
        .mark_running_as_interrupted(Timestamp::from_unix_millis(99))
        .expect("start-up mark");

    assert_eq!(changed, 0);
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

#[test]
fn a_worktree_survives_a_reopen() {
    let folder = tempfile::tempdir().expect("folder");
    let database = database_in(&folder);
    let first = open_office(&database);
    let mut agent = agent_named(&first, "Ana", None);
    agent.worktree = Some(agent_domain::value_objects::AgentWorktree {
        repository: PathBuf::from("/work/repo"),
        base: "main".to_owned(),
        branch: "office-harness/ana".to_owned(),
        folder: PathBuf::from("/data/worktrees/repo-1234abcd/0a1b2c3d"),
    });
    first.agents.insert(&agent).expect("agent");
    let plain = agent_named(&first, "Ben", None);
    first.agents.insert(&plain).expect("agent");
    drop(first);

    let second = open_office(&database);

    assert_eq!(second.agents.list().expect("agents"), [agent, plain]);
}

fn temp_mark(creator: Actor) -> TempMark {
    TempMark {
        objective: Objective::new("fix the failing build").expect("objective"),
        creator,
    }
}

#[test]
fn a_temp_mark_survives_a_reopen_for_each_kind_of_creator() {
    let folder = tempfile::tempdir().expect("folder");
    let database = database_in(&folder);
    let first = open_office(&database);
    let mut by_user = agent_named(&first, "Ana", None);
    by_user.temp = Some(temp_mark(Actor::User));
    let mut by_agent = agent_named(&first, "Ben", None);
    by_agent.temp = Some(temp_mark(Actor::Agent(AgentId::generate())));
    let plain = agent_named(&first, "Cem", None);
    for agent in [&by_user, &by_agent, &plain] {
        first.agents.insert(agent).expect("agent");
    }
    drop(first);

    let second = open_office(&database);

    assert_eq!(
        second.agents.list().expect("agents"),
        [by_user, by_agent, plain]
    );
}

#[test]
fn a_row_with_only_one_temp_column_reads_as_no_temp() {
    let folder = tempfile::tempdir().expect("folder");
    let database = database_in(&folder);
    let first = open_office(&database);
    let agent = agent_named(&first, "Ana", None);
    first.agents.insert(&agent).expect("agent");
    drop(first);
    Connection::open(&database)
        .expect("open")
        .execute("UPDATE agent_agents SET temp_objective = 'half'", [])
        .expect("update");

    let second = open_office(&database);

    assert_eq!(second.agents.list().expect("agents"), [agent]);
}

fn finished_temp(name: &str, ended_at: i64) -> FinishedTemp {
    FinishedTemp {
        id: FinishedTempId::generate(),
        agent_id: AgentId::generate(),
        name: name.to_owned(),
        objective: "fix the failing build".to_owned(),
        creator: Actor::User,
        parent_name: None,
        started_at: Timestamp::from_unix_millis(1),
        ended_at: Timestamp::from_unix_millis(ended_at),
        reason: "the temp was quiet for 30 minutes".to_owned(),
        report: None,
        kept_folder: None,
        kept_branch: None,
        removal_failure: None,
    }
}

#[test]
fn finished_temps_list_newest_first_and_survive_a_reopen() {
    let folder = tempfile::tempdir().expect("folder");
    let database = database_in(&folder);
    let first = open_office(&database);
    let oldest = finished_temp("Old", 10);
    let mut newest = finished_temp("New", 30);
    newest.creator = Actor::Agent(AgentId::generate());
    newest.parent_name = Some("Lead".to_owned());
    newest.report = Some(FinishedReport {
        kind: "done".to_owned(),
        text: "all green".to_owned(),
    });
    newest.kept_folder = Some(PathBuf::from("/data/worktrees/repo/0a1b"));
    newest.kept_branch = Some("office-harness/new".to_owned());
    newest.removal_failure = Some("git refused".to_owned());
    let middle = finished_temp("Mid", 20);
    for entry in [&oldest, &newest, &middle] {
        first.finished_temps.insert(entry).expect("insert");
    }
    drop(first);

    let second = open_office(&database);

    assert_eq!(
        second.finished_temps.list().expect("list"),
        [newest, middle, oldest]
    );
}

fn process_of(pid: u32, started_at: u64, command: &str) -> SessionProcess {
    SessionProcess {
        pid,
        started_at,
        command_hash: [u8::try_from(pid).expect("small pid"); 32],
        command_display: command.to_owned(),
        boot: "boot-1".to_owned(),
    }
}

/// An office with one agent whose session runs.
fn office_with_running_session(folder: &TempDir) -> (Office, Agent, Session) {
    let office = open_office(&database_in(folder));
    let agent = agent_named(&office, "Ana", None);
    office.agents.insert(&agent).expect("agent");
    let session = session_of(&agent, 10, OfficeAdditions::default());
    office.sessions.insert(&session).expect("session");
    (office, agent, session)
}

#[test]
fn a_process_is_stored_open_and_a_second_row_for_the_same_run_is_not_stored() {
    let folder = TempDir::new().expect("folder");
    let (office, _agent, session) = office_with_running_session(&folder);
    let process = process_of(7, 100, "claude");

    let first = office
        .processes
        .insert_open(session.id, ProcessKind::Cli, &process)
        .expect("insert");
    let second = office
        .processes
        .insert_open(session.id, ProcessKind::Helper, &process)
        .expect("same key");

    assert!(first);
    assert!(!second);
    let rows = office.processes.open_for_session(session.id).expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, ProcessKind::Cli);
    assert_eq!(rows[0].process, process);
}

#[test]
fn a_session_that_ended_takes_no_new_row() {
    let folder = TempDir::new().expect("folder");
    let (office, agent, session) = office_with_running_session(&folder);
    office
        .sessions
        .end_running(
            agent.id,
            &ExitResult::Signal,
            Timestamp::from_unix_millis(20),
        )
        .expect("end");

    let stored = office
        .processes
        .insert_open(session.id, ProcessKind::Helper, &process_of(7, 100, "node"))
        .expect("insert");

    assert!(!stored);
}

#[test]
fn a_replaced_command_line_updates_the_row_in_place_and_keeps_it_open() {
    let folder = TempDir::new().expect("folder");
    let (office, _agent, session) = office_with_running_session(&folder);
    office
        .processes
        .insert_open(
            session.id,
            ProcessKind::Helper,
            &process_of(7, 100, "sh -c"),
        )
        .expect("insert");
    let row = office.processes.open_for_session(session.id).expect("rows")[0].clone();

    let exec = process_of(7, 100, "node server.js");
    office
        .processes
        .update_command(row.id, &exec)
        .expect("update");

    let rows = office.processes.open_for_session(session.id).expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, row.id);
    assert_eq!(rows[0].process.command_display, "node server.js");
    assert_eq!(rows[0].process.command_hash, exec.command_hash);
}

#[test]
fn dropping_removes_a_helper_row_and_never_a_cli_row() {
    let folder = TempDir::new().expect("folder");
    let (office, _agent, session) = office_with_running_session(&folder);
    for (pid, kind) in [(7, ProcessKind::Cli), (8, ProcessKind::Helper)] {
        office
            .processes
            .insert_open(session.id, kind, &process_of(pid, 100, "x"))
            .expect("insert");
    }

    for row in office.processes.open_for_session(session.id).expect("rows") {
        office.processes.drop_helper(row.id).expect("drop");
    }

    let rows = office.processes.open_for_session(session.id).expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, ProcessKind::Cli);
}

#[test]
fn ending_one_row_leaves_the_others_open() {
    let folder = TempDir::new().expect("folder");
    let (office, _agent, session) = office_with_running_session(&folder);
    for pid in [7, 8] {
        office
            .processes
            .insert_open(session.id, ProcessKind::Helper, &process_of(pid, 100, "x"))
            .expect("insert");
    }
    let first = office.processes.open_for_session(session.id).expect("rows")[0].id;

    office
        .processes
        .end_one(first, SweepOutcome::Left)
        .expect("end one");

    let rows = office.processes.open_for_session(session.id).expect("rows");
    assert_eq!(rows.len(), 1);
    assert_ne!(rows[0].id, first);
}

#[test]
fn ending_a_session_ends_all_its_rows_and_only_its_own() {
    let folder = TempDir::new().expect("folder");
    let (office, _agent, session) = office_with_running_session(&folder);
    let other_agent = agent_named(&office, "Ben", None);
    office.agents.insert(&other_agent).expect("agent");
    let other = session_of(&other_agent, 11, OfficeAdditions::default());
    office.sessions.insert(&other).expect("session");
    for id in [session.id, other.id] {
        office
            .processes
            .insert_open(id, ProcessKind::Cli, &process_of(7, 100, "x"))
            .expect("insert");
    }

    office.processes.end_for_session(session.id).expect("end");

    assert_eq!(
        office.processes.open_for_session(session.id).expect("rows"),
        Vec::new()
    );
    assert_eq!(
        office
            .processes
            .open_for_session(other.id)
            .expect("rows")
            .len(),
        1
    );
}

#[test]
fn a_dismiss_removes_one_entry_and_nothing_else() {
    let folder = tempfile::tempdir().expect("folder");
    let office = open_office(&database_in(&folder));
    let kept = finished_temp("Kept", 10);
    let dismissed = finished_temp("Gone", 20);
    office.finished_temps.insert(&kept).expect("insert");
    office.finished_temps.insert(&dismissed).expect("insert");

    assert!(
        office
            .finished_temps
            .dismiss(dismissed.id)
            .expect("dismiss")
    );
    assert!(!office.finished_temps.dismiss(dismissed.id).expect("again"));

    assert_eq!(office.finished_temps.list().expect("list"), [kept]);
}

fn stored_rows_of(folder: &TempDir, session_id: SessionId) -> i64 {
    Connection::open(database_in(folder))
        .expect("open database")
        .query_row(
            "SELECT count(*) FROM agent_session_processes WHERE session_id = ?1",
            [session_id.to_string()],
            |row| row.get(0),
        )
        .expect("count")
}

#[test]
fn every_normal_end_of_a_running_session_deletes_its_rows_in_the_same_step() {
    let normal_ends = [
        ExitResult::Signal,
        ExitResult::Code(1),
        ExitResult::SpawnError("no such file".to_owned()),
        ExitResult::NotSignedIn("sign in".to_owned()),
        ExitResult::Interrupted,
    ];
    for result in normal_ends {
        let folder = TempDir::new().expect("folder");
        let (office, agent, session) = office_with_running_session(&folder);
        for (pid, kind) in [(7, ProcessKind::Cli), (8, ProcessKind::Helper)] {
            office
                .processes
                .insert_open(session.id, kind, &process_of(pid, 100, "x"))
                .expect("insert");
        }

        office
            .sessions
            .end_running(agent.id, &result, Timestamp::from_unix_millis(20))
            .expect("end");

        assert_eq!(stored_rows_of(&folder, session.id), 0, "{result}");
        assert_eq!(
            office.processes.open_for_lost_sessions().expect("lost"),
            Vec::new()
        );
    }
}

#[test]
fn a_session_end_deletes_the_rows_of_its_own_session_only() {
    let folder = TempDir::new().expect("folder");
    let (office, agent, session) = office_with_running_session(&folder);
    let other_agent = agent_named(&office, "Ben", None);
    office.agents.insert(&other_agent).expect("agent");
    let other = session_of(&other_agent, 11, OfficeAdditions::default());
    office.sessions.insert(&other).expect("session");
    for id in [session.id, other.id] {
        office
            .processes
            .insert_open(id, ProcessKind::Cli, &process_of(7, 100, "x"))
            .expect("insert");
    }

    office
        .sessions
        .end_running(
            agent.id,
            &ExitResult::Signal,
            Timestamp::from_unix_millis(20),
        )
        .expect("end");

    assert_eq!(stored_rows_of(&folder, session.id), 0);
    assert_eq!(stored_rows_of(&folder, other.id), 1);
}

#[test]
fn a_new_session_deletes_the_ended_rows_of_earlier_lost_sessions_and_keeps_open_ones() {
    let folder = TempDir::new().expect("folder");
    let (office, agent, lost) = office_with_running_session(&folder);
    for pid in [7, 8] {
        office
            .processes
            .insert_open(lost.id, ProcessKind::Helper, &process_of(pid, 100, "x"))
            .expect("insert");
    }
    let swept = office.processes.open_for_session(lost.id).expect("rows")[0].id;
    office
        .processes
        .end_one(swept, SweepOutcome::Ended)
        .expect("end one");
    office
        .sessions
        .mark_running_as_interrupted(Timestamp::from_unix_millis(20))
        .expect("lose");

    office
        .sessions
        .insert(&session_of(&agent, 30, OfficeAdditions::default()))
        .expect("new session");

    let kept = office.processes.open_for_lost_sessions().expect("lost");
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].process.pid, 8);
    assert_eq!(stored_rows_of(&folder, lost.id), 1);
}

#[test]
fn a_new_session_leaves_the_ended_rows_of_another_agents_lost_session() {
    let folder = TempDir::new().expect("folder");
    let (office, _agent, lost) = office_with_running_session(&folder);
    office
        .processes
        .insert_open(lost.id, ProcessKind::Cli, &process_of(7, 100, "x"))
        .expect("insert");
    office.processes.end_for_session(lost.id).expect("end");
    office
        .sessions
        .mark_running_as_interrupted(Timestamp::from_unix_millis(20))
        .expect("lose");
    let other_agent = agent_named(&office, "Ben", None);
    office.agents.insert(&other_agent).expect("agent");

    office
        .sessions
        .insert(&session_of(&other_agent, 30, OfficeAdditions::default()))
        .expect("new session");

    assert_eq!(stored_rows_of(&folder, lost.id), 1);
}

#[test]
fn open_rows_of_lost_sessions_are_listed_and_no_others() {
    let folder = TempDir::new().expect("folder");
    let (office, _agent, lost) = office_with_running_session(&folder);
    let other_agent = agent_named(&office, "Ben", None);
    office.agents.insert(&other_agent).expect("agent");
    let still_running = session_of(&other_agent, 11, OfficeAdditions::default());
    office.sessions.insert(&still_running).expect("session");
    for id in [lost.id, still_running.id] {
        office
            .processes
            .insert_open(id, ProcessKind::Cli, &process_of(7, 100, "x"))
            .expect("insert");
    }
    office
        .processes
        .insert_open(lost.id, ProcessKind::Helper, &process_of(8, 100, "y"))
        .expect("insert");
    let helper = office.processes.open_for_session(lost.id).expect("rows")[1].id;
    office
        .processes
        .end_one(helper, SweepOutcome::Left)
        .expect("end one");
    // Only the first session is marked lost by the next start.
    Connection::open(database_in(&folder))
        .expect("open database")
        .execute(
            "UPDATE agent_sessions SET ended_at = 30, exit_result = 'lost' WHERE id = ?1",
            [lost.id.to_string()],
        )
        .expect("lose the session");

    let rows = office.processes.open_for_lost_sessions().expect("lost");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].session_id, lost.id);
    assert_eq!(rows[0].process.pid, 7);
}

#[test]
fn a_command_display_over_the_cap_is_refused_by_the_table() {
    let folder = TempDir::new().expect("folder");
    let (office, _agent, session) = office_with_running_session(&folder);
    let within = "x".repeat(1_000);
    let beyond = "x".repeat(1_001);

    let stored = office
        .processes
        .insert_open(session.id, ProcessKind::Cli, &process_of(7, 100, &within))
        .expect("at the cap");
    let refused =
        office
            .processes
            .insert_open(session.id, ProcessKind::Cli, &process_of(8, 100, &beyond));

    assert!(stored);
    assert!(matches!(refused, Err(PersistenceError::Storage(_))));
}

#[test]
fn a_windows_file_time_round_trips_through_the_database_integer() {
    let folder = TempDir::new().expect("folder");
    let (office, _agent, session) = office_with_running_session(&folder);
    let file_time = 133_500_000_123_456_789_u64;

    office
        .processes
        .insert_open(session.id, ProcessKind::Cli, &process_of(7, file_time, "x"))
        .expect("insert");

    let rows = office.processes.open_for_session(session.id).expect("rows");
    assert_eq!(rows[0].process.started_at, file_time);
}

#[test]
fn a_start_time_beyond_the_database_integer_is_an_error_not_a_wrapped_value() {
    let folder = TempDir::new().expect("folder");
    let (office, _agent, session) = office_with_running_session(&folder);

    let stored =
        office
            .processes
            .insert_open(session.id, ProcessKind::Cli, &process_of(7, u64::MAX, "x"));

    assert!(matches!(stored, Err(PersistenceError::Storage(_))));
}

#[test]
fn the_helper_limit_mark_is_kept_on_the_session() {
    let folder = TempDir::new().expect("folder");
    let (office, _agent, session) = office_with_running_session(&folder);

    office
        .processes
        .mark_helper_limit(session.id)
        .expect("mark");

    let marked: i64 = Connection::open(database_in(&folder))
        .expect("open database")
        .query_row(
            "SELECT helpers_capped FROM agent_sessions WHERE id = ?1",
            [session.id.to_string()],
            |row| row.get(0),
        )
        .expect("column");
    assert_eq!(marked, 1);
}

#[test]
fn the_sweep_summary_counts_each_outcome_and_the_rows_left_open() {
    let folder = TempDir::new().expect("folder");
    let (office, _agent, session) = office_with_running_session(&folder);
    let outcomes = [
        SweepOutcome::Ended,
        SweepOutcome::Ended,
        SweepOutcome::CouldNotEnd,
        SweepOutcome::NotReadable,
        SweepOutcome::BootTimeChanged,
        SweepOutcome::Left,
    ];
    for pid in 1..=7 {
        office
            .processes
            .insert_open(session.id, ProcessKind::Helper, &process_of(pid, 100, "x"))
            .expect("insert");
    }
    let rows = office.processes.open_for_session(session.id).expect("rows");
    for (row, outcome) in rows.iter().zip(outcomes) {
        office.processes.end_one(row.id, outcome).expect("end one");
    }
    office
        .processes
        .mark_helper_limit(session.id)
        .expect("mark");

    let summary = office.processes.sweep_summary(session.id).expect("summary");

    assert_eq!(
        summary,
        SweepSummary {
            ended: 2,
            could_not_end: 1,
            not_readable: 1,
            boot_time_changed: 1,
            still_open: 1,
            cli_rows: 0,
            helpers_capped: true,
        }
    );
}

#[test]
fn the_sweep_summary_counts_the_cli_row_and_not_the_helpers() {
    let folder = TempDir::new().expect("folder");
    let (office, _agent, session) = office_with_running_session(&folder);
    office
        .processes
        .insert_open(session.id, ProcessKind::Helper, &process_of(8, 100, "x"))
        .expect("helper");
    let helpers_only = office.processes.sweep_summary(session.id).expect("summary");

    office
        .processes
        .insert_open(session.id, ProcessKind::Cli, &process_of(7, 100, "y"))
        .expect("cli");
    let with_cli = office.processes.sweep_summary(session.id).expect("summary");

    assert_eq!((helpers_only.cli_rows, with_cli.cli_rows), (0, 1));
}

#[test]
fn the_sweep_summary_of_a_session_without_rows_is_empty() {
    let folder = TempDir::new().expect("folder");
    let (office, _agent, session) = office_with_running_session(&folder);

    let summary = office.processes.sweep_summary(session.id).expect("summary");

    assert_eq!(summary, SweepSummary::default());
}

#[test]
fn a_session_end_leaves_no_sweep_outcome() {
    let folder = TempDir::new().expect("folder");
    let (office, _agent, session) = office_with_running_session(&folder);
    office
        .processes
        .insert_open(session.id, ProcessKind::Cli, &process_of(7, 100, "x"))
        .expect("insert");
    office.processes.end_for_session(session.id).expect("end");

    let summary = office.processes.sweep_summary(session.id).expect("summary");

    assert_eq!(
        (summary.cli_rows, summary.ended, summary.still_open),
        (1, 0, 0)
    );
}
