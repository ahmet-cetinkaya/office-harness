// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::ExitResult;
use tempfile::TempDir;

use super::*;
use crate::common::interfaces::SessionStore;
use crate::test_support::{Office, absolute_folder};

#[test]
fn a_stopped_agent_moves_to_starting_and_its_command_is_split() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent(
        "Ana",
        "claude --model opus",
        folder.path(),
        AgentStatus::Stopped,
    );

    let outcome = office.start_agent().handle(agent.id).expect("start");

    assert_eq!(outcome.agent.status, AgentStatus::Starting);
    assert_eq!(outcome.failure, None);
    assert_eq!(
        office.agents.status_of(agent.id),
        Some(AgentStatus::Starting)
    );
    let started = office.terminals.started();
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].program, "claude");
    // The adapter adds the office session id after the user's arguments.
    assert_eq!(
        started[0].arguments[..3],
        ["--model", "opus", "--session-id"]
    );
    assert_eq!(started[0].working_folder, folder.path());
}

#[test]
fn a_start_adds_one_entry_with_the_user_as_actor() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);

    office.start_agent().handle(agent.id).expect("start");

    let entries = office.recorder.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, AgentAction::Start);
    assert_eq!(entries[0].agent_id, agent.id);
    assert_eq!(entries[0].actor, kernel::Actor::User);
    assert_eq!(entries[0].failure, None);
}

#[test]
fn a_command_that_cannot_start_fails_that_agent_with_the_error_text() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let broken = office.add_agent("Broken", "nonesuch", folder.path(), AgentStatus::Stopped);
    let healthy = office.add_agent("Healthy", "claude", folder.path(), AgentStatus::Working);
    office.terminals.make_unstartable("nonesuch");

    let outcome = office.start_agent().handle(broken.id).expect("accepted");

    assert_eq!(outcome.agent.status, AgentStatus::Failed);
    assert!(
        outcome
            .failure
            .as_deref()
            .is_some_and(|text| text.contains("nonesuch"))
    );
    assert_eq!(
        office.agents.status_of(broken.id),
        Some(AgentStatus::Failed)
    );
    assert_eq!(
        office.agents.status_of(healthy.id),
        Some(AgentStatus::Working)
    );
    assert!(office.recorder.entries()[0].failure.is_some());
}

#[test]
fn two_agents_start_side_by_side() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let first = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);
    let second = office.add_agent("Ben", "codex", folder.path(), AgentStatus::Stopped);

    office.start_agent().handle(first.id).expect("first");
    office.start_agent().handle(second.id).expect("second");

    assert_eq!(
        office.agents.status_of(first.id),
        Some(AgentStatus::Starting)
    );
    assert_eq!(
        office.agents.status_of(second.id),
        Some(AgentStatus::Starting)
    );
    assert_eq!(office.terminals.started().len(), 2);
}

#[test]
fn a_missing_folder_refuses_the_start_and_names_it() {
    let office = Office::new();
    let missing = absolute_folder("office-harness-no-such-folder");
    let agent = office.add_agent("Ana", "claude", &missing, AgentStatus::Stopped);

    let error = office.start_agent().handle(agent.id).expect_err("missing");

    assert!(matches!(&error, ApplicationError::Invalid(text)
        if text.contains("working folder") && text.contains("office-harness-no-such-folder")));
    assert_eq!(
        office.agents.status_of(agent.id),
        Some(AgentStatus::Stopped)
    );
    assert_eq!(office.terminals.started().len(), 0);
    assert!(office.recorder.entries()[0].failure.is_some());
}

#[test]
fn a_running_agent_cannot_start_again() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Working);

    let error = office.start_agent().handle(agent.id).expect_err("running");

    assert!(matches!(&error, ApplicationError::Conflict(text) if text.contains("Ana")));
    assert_eq!(office.terminals.started().len(), 0);
}

#[test]
fn an_interrupted_agent_starts_only_on_request() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Interrupted);

    let outcome = office.start_agent().handle(agent.id).expect("start");

    assert_eq!(outcome.agent.status, AgentStatus::Starting);
}

#[test]
fn an_unknown_agent_is_not_found_and_leaves_no_entry() {
    let office = Office::new();

    let error = office
        .start_agent()
        .handle(AgentId::generate())
        .expect_err("unknown");

    assert!(matches!(error, ApplicationError::NotFound(_)));
    assert_eq!(office.recorder.entries().len(), 0);
}

#[test]
fn a_quoted_argument_with_a_space_reaches_the_terminal_as_one_argument() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent(
        "Ana",
        r#"claude --append-system-prompt "be brief""#,
        folder.path(),
        AgentStatus::Stopped,
    );

    office.start_agent().handle(agent.id).expect("start");

    assert_eq!(
        office.terminals.started()[0].arguments[..2],
        ["--append-system-prompt", "be brief"]
    );
}

fn set_additions(office: &Office, agent: AgentId, flags: &[&str], key: &str, prompt: &str) {
    office
        .set_additions()
        .handle(
            agent,
            crate::common::additions_input::AdditionsInput {
                flags: flags.iter().map(|flag| (*flag).to_owned()).collect(),
                env: vec![(key.to_owned(), "value".to_owned())],
                startup_prompt: Some(prompt.to_owned()),
            },
        )
        .expect("additions");
}

#[test]
fn flags_env_and_the_startup_prompt_reach_the_terminal_and_the_session_records_them() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent(
        "Ana",
        "claude --model opus",
        folder.path(),
        AgentStatus::Stopped,
    );
    set_additions(
        &office,
        agent.id,
        &["--verbose"],
        "OFFICE_KEY",
        "do the thing",
    );

    let outcome = office.start_agent().handle(agent.id).expect("start");

    let started = &office.terminals.started()[0];
    assert_eq!(
        started.arguments[..3],
        ["--model", "opus", "--verbose"],
        "fixed arguments first, then the flags"
    );
    assert_eq!(started.typed_input.as_deref(), Some("do the thing"));
    assert!(!started.arguments.contains(&"do the thing".to_owned()));
    assert!(
        started
            .environment
            .contains(&("OFFICE_KEY".to_owned(), "value".to_owned()))
    );
    assert_eq!(started.session_id, outcome.session.id);
    let snapshot = &outcome.session.additions_snapshot;
    assert_eq!(snapshot.flags(), ["--verbose"]);
    assert_eq!(snapshot.startup_prompt(), Some("do the thing"));
}

#[test]
fn an_edit_during_a_session_leaves_that_session_unchanged_and_applies_to_the_next() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);
    set_additions(&office, agent.id, &["--first"], "FIRST", "one");
    let running = office
        .start_agent()
        .handle(agent.id)
        .expect("start")
        .session;

    set_additions(&office, agent.id, &["--second"], "SECOND", "two");

    let still = office
        .sessions
        .find(running.id)
        .expect("find")
        .expect("stored");
    assert_eq!(still.additions_snapshot.flags(), ["--first"]);
    assert_eq!(office.terminals.started().len(), 1);
    let next = office.start_new_task().handle(agent.id).expect("new task");
    assert_eq!(next.session.additions_snapshot.flags(), ["--second"]);
    let second_start = &office.terminals.started()[1];
    assert!(second_start.arguments.contains(&"--second".to_owned()));
    assert!(
        second_start
            .environment
            .iter()
            .any(|(name, _)| name == "SECOND")
    );
    assert!(
        !second_start
            .environment
            .iter()
            .any(|(name, _)| name == "FIRST")
    );
}

#[test]
fn a_flag_or_a_fixed_argument_with_a_percent_is_refused_for_a_batch_script() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    office.terminals.make_batch_script("shim");
    let by_flag = office.add_agent("Ana", "shim", folder.path(), AgentStatus::Stopped);
    set_additions(&office, by_flag.id, &["--path=%PATH%"], "K", "p");
    let by_fixed = office.add_agent(
        "Ben",
        "shim --x=%PATH%",
        folder.path(),
        AgentStatus::Stopped,
    );

    for agent in [by_flag.id, by_fixed.id] {
        let error = office.start_agent().handle(agent).expect_err("refused");
        assert!(matches!(&error, ApplicationError::Invalid(text) if text.contains('%')));
        assert_eq!(office.agents.status_of(agent), Some(AgentStatus::Stopped));
    }
    assert_eq!(office.terminals.started().len(), 0);
    assert_eq!(office.sessions.all().len(), 0);
}

#[test]
fn a_startup_prompt_is_typed_in_for_a_batch_script_and_never_put_on_the_command_line() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    office.terminals.make_batch_script("shim");
    let agent = office.add_agent("Ana", "shim", folder.path(), AgentStatus::Stopped);
    let prompt = "echo %PATH% & calc\nsecond line";
    set_additions(&office, agent.id, &[], "K", prompt);

    office.start_agent().handle(agent.id).expect("start");

    let started = &office.terminals.started()[0];
    assert_eq!(started.typed_input.as_deref(), Some(prompt));
    assert!(
        started
            .arguments
            .iter()
            .all(|argument| !argument.contains("calc"))
    );
}

#[test]
fn a_command_that_cannot_start_leaves_a_session_that_never_started() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Broken", "nonesuch", folder.path(), AgentStatus::Stopped);
    office.terminals.make_unstartable("nonesuch");

    let outcome = office.start_agent().handle(agent.id).expect("accepted");

    let stored = office.sessions.all();
    assert_eq!(stored.len(), 1);
    assert!(
        matches!(&stored[0].exit_result, Some(ExitResult::SpawnError(text)) if text.contains("nonesuch"))
    );
    assert_eq!(stored[0].id, outcome.session.id);
    assert!(stored[0].ended_at.is_some());
}

#[test]
fn a_cli_that_does_not_take_an_id_learns_it_after_the_start() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "tracked", folder.path(), AgentStatus::Stopped);

    let outcome = office.start_agent().handle(agent.id).expect("start");

    let stored = office
        .sessions
        .find(outcome.session.id)
        .expect("find")
        .expect("stored");
    assert_eq!(stored.cli_session_id.as_deref(), Some("tracked-id"));
}

#[test]
fn a_cli_that_takes_an_id_gets_the_one_the_session_records() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);

    let outcome = office.start_agent().handle(agent.id).expect("start");

    let proposed = outcome.session.cli_session_id.clone().expect("id");
    assert!(office.terminals.started()[0].arguments.contains(&proposed));
}
