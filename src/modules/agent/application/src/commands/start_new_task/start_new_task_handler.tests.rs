// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{ExitResult, TaskSource, TaskText};
use tempfile::TempDir;

use super::*;
use crate::common::interfaces::SessionStore;
use crate::test_support::Office;

#[test]
fn a_new_task_on_a_running_agent_ends_the_running_session_and_starts_a_new_one() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);
    let first = office
        .start_agent()
        .handle(agent.id)
        .expect("start")
        .session;
    office
        .agents
        .update_status(agent.id, AgentStatus::Working)
        .expect("status");

    let second = office.start_new_task().handle(agent.id).expect("new task");

    assert_ne!(first.id, second.session.id);
    assert_eq!(second.agent.status, AgentStatus::Starting);
    assert_eq!(second.session.continued_from, None);
    let stored = office
        .sessions
        .find(first.id)
        .expect("find")
        .expect("stored");
    assert_eq!(stored.exit_result, Some(ExitResult::Signal));
    assert_eq!(office.terminals.stopped().len(), 1);
    assert_eq!(office.terminals.started().len(), 2);
}

#[test]
fn a_new_task_on_a_stopped_agent_starts_a_session_without_stopping_anything() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Exited);

    office.start_new_task().handle(agent.id).expect("new task");

    assert_eq!(office.terminals.stopped().len(), 0);
    assert_eq!(office.sessions.all().len(), 1);
}

#[test]
fn a_new_task_adds_one_new_task_entry() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);

    office.start_new_task().handle(agent.id).expect("new task");

    let entries = office.recorder.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, AgentAction::NewTask);
    assert_eq!(entries[0].failure, None);
}

#[test]
fn a_new_task_that_is_refused_leaves_the_running_session_running() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);
    let first = office
        .start_agent()
        .handle(agent.id)
        .expect("start")
        .session;
    office
        .agents
        .update_status(agent.id, AgentStatus::Working)
        .expect("status");
    drop(folder);

    let error = office
        .start_new_task()
        .handle(agent.id)
        .expect_err("the folder is gone");

    assert!(matches!(error, ApplicationError::Invalid(_)), "{error:?}");
    assert_eq!(office.terminals.stopped().len(), 0);
    assert_eq!(
        office.agents.status_of(agent.id),
        Some(AgentStatus::Working)
    );
    let stored = office
        .sessions
        .find(first.id)
        .expect("find")
        .expect("stored");
    assert_eq!(stored.exit_result, None);
}

#[test]
fn a_new_task_after_the_child_ended_on_its_own_closes_that_session_and_starts_the_next() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);
    let first = office
        .start_agent()
        .handle(agent.id)
        .expect("start")
        .session;
    office
        .agents
        .update_status(agent.id, AgentStatus::Working)
        .expect("status");
    office.terminals.end_children_on_their_own();

    let outcome = office
        .start_new_task()
        .handle(agent.id)
        .expect("no false conflict");

    let stored = office
        .sessions
        .find(first.id)
        .expect("find")
        .expect("stored");
    assert_eq!(stored.exit_result, Some(ExitResult::Signal));
    assert_eq!(outcome.failure, None);
    assert_eq!(office.sessions.all().len(), 2);
    assert_eq!(office.terminals.started().len(), 2);
}

fn give_task(office: &Office, agent: &agent_domain::entities::Agent) {
    let task = TaskText::from_input(
        "Fix the login test",
        kernel::Timestamp::from_unix_millis(5),
        TaskSource::Directive,
    )
    .expect("valid")
    .expect("some");
    office
        .agents
        .update_task(agent.id, Some(&task))
        .expect("task");
}

fn task_of(office: &Office, agent: &agent_domain::entities::Agent) -> Option<TaskText> {
    office
        .agents
        .find(agent.id)
        .expect("find")
        .expect("agent")
        .task
}

#[test]
fn a_new_task_clears_the_task_text_after_the_session_started() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);
    give_task(&office, &agent);

    let outcome = office.start_new_task().handle(agent.id).expect("new task");

    assert_eq!(outcome.agent.task, None);
    assert_eq!(task_of(&office, &agent), None);
}

#[test]
fn a_new_task_whose_command_cannot_start_keeps_the_task_text() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Broken", "nonesuch", folder.path(), AgentStatus::Stopped);
    office.terminals.make_unstartable("nonesuch");
    give_task(&office, &agent);

    let outcome = office.start_new_task().handle(agent.id).expect("accepted");

    assert!(outcome.failure.is_some());
    assert!(task_of(&office, &agent).is_some());
}

#[test]
fn starting_and_continuing_keep_the_task_text() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);
    give_task(&office, &agent);

    let first = office.start_agent().handle(agent.id).expect("start");
    assert!(task_of(&office, &agent).is_some());
    office
        .end_session()
        .handle(agent.id, &ExitResult::Code(0))
        .expect("end");
    office
        .agents
        .update_status(agent.id, AgentStatus::Exited)
        .expect("status");
    office
        .continue_session()
        .handle(first.session.id)
        .expect("continue");

    assert!(task_of(&office, &agent).is_some());
}
