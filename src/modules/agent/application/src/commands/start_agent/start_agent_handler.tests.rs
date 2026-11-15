// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::enums::AgentStatus;
use tempfile::TempDir;

use super::*;
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
    assert_eq!(started[0].arguments, ["--model", "opus"]);
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
        office.terminals.started()[0].arguments,
        ["--append-system-prompt", "be brief"]
    );
}
