// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use agent_domain::enums::AgentStatus;
use tempfile::TempDir;

use super::*;
use crate::common::interfaces::SessionStore;
use crate::test_support::Office;

#[test]
fn a_running_agent_stops_with_the_grace_period_and_adds_an_entry() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Working);

    let stopped = office.stop_agent().handle(agent.id).expect("stop");

    assert_eq!(stopped.status, AgentStatus::Stopped);
    assert_eq!(
        office.agents.status_of(agent.id),
        Some(AgentStatus::Stopped)
    );
    let grace = Duration::from_millis(office.settings.current().stop_grace_ms);
    assert_eq!(office.terminals.stopped(), [(agent.id, grace)]);
    let entries = office.recorder.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, AgentAction::Stop);
    assert_eq!(entries[0].failure, None);
}

#[test]
fn an_agent_that_does_not_run_conflicts_and_the_refusal_is_recorded() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Exited);

    let error = office
        .stop_agent()
        .handle(agent.id)
        .expect_err("not running");

    assert!(matches!(&error, ApplicationError::Conflict(text) if text.contains("Ana")));
    assert_eq!(office.terminals.stopped().len(), 0);
    assert!(office.recorder.entries()[0].failure.is_some());
}

#[test]
fn stopping_one_agent_leaves_the_other_running() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let first = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Working);
    let second = office.add_agent("Ben", "codex", folder.path(), AgentStatus::WaitingForInput);

    office.stop_agent().handle(first.id).expect("stop");

    assert_eq!(
        office.agents.status_of(second.id),
        Some(AgentStatus::WaitingForInput)
    );
}

#[test]
fn a_child_that_ended_on_its_own_leaves_the_status_to_its_exit_event() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Working);
    office.terminals.end_children_on_their_own();

    let unchanged = office.stop_agent().handle(agent.id).expect("no-op success");

    assert_eq!(unchanged.status, AgentStatus::Working);
    assert_eq!(
        office.agents.status_of(agent.id),
        Some(AgentStatus::Working)
    );
    assert_eq!(office.recorder.entries()[0].failure, None);
}

#[test]
fn a_stop_closes_the_running_session_with_signal() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);
    let session = office
        .start_agent()
        .handle(agent.id)
        .expect("start")
        .session;

    office.stop_agent().handle(agent.id).expect("stop");

    let stored = office
        .sessions
        .find(session.id)
        .expect("find")
        .expect("stored");
    assert_eq!(
        stored.exit_result,
        Some(agent_domain::value_objects::ExitResult::Signal)
    );
}
