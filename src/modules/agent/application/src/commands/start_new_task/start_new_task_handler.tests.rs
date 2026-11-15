// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::ExitResult;
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
