// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::value_objects::ExitResult;
use kernel::Actor;
use tempfile::TempDir;

use super::*;
use crate::common::actor_scope::ActorScope;
use crate::common::interfaces::SessionStore;
use crate::test_support::Office;

/// An agent that waits for input in a running session.
fn waiting_agent(office: &Office) -> (Agent, agent_domain::value_objects::SessionId) {
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent(
        "Ana",
        "claude",
        folder.keep().as_path(),
        AgentStatus::Stopped,
    );
    let session = office
        .start_agent()
        .handle(agent.id)
        .expect("start")
        .session;
    office
        .agents
        .update_status(agent.id, AgentStatus::WaitingForInput)
        .expect("status");
    (agent, session.id)
}

#[test]
fn a_waiting_agent_sleeps_with_one_office_entry_and_its_session_ends() {
    let office = Office::new();
    let (agent, session_id) = waiting_agent(&office);
    let entries_before = office.recorder.entries().len();
    let _scope = ActorScope::enter(Actor::Office);

    let slept = office.sleep_agent().handle(agent.id, 30).expect("sleep");

    assert_eq!(slept.status, AgentStatus::Asleep);
    assert_eq!(office.agents.status_of(agent.id), Some(AgentStatus::Asleep));
    assert_eq!(office.terminals.stopped().len(), 1);
    let entries = office.recorder.entries();
    assert_eq!(entries.len(), entries_before + 1);
    let entry = entries.last().expect("entry");
    assert_eq!(entry.action, AgentAction::Sleep);
    assert_eq!(entry.actor, Actor::Office);
    assert_eq!(entry.failure, None);
    assert_eq!(entry.detail.as_deref(), Some("idle for 30 minutes"));
    let stored = office
        .sessions
        .find(session_id)
        .expect("find")
        .expect("stored");
    assert_eq!(stored.exit_result, Some(ExitResult::Signal));
    assert_eq!(office.sleep_gate.calls(), ["begin", "end"]);
}

#[test]
fn an_agent_that_does_not_wait_for_input_conflicts_without_a_record_or_a_stop() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Working);

    let error = office
        .sleep_agent()
        .handle(agent.id, 30)
        .expect_err("not waiting");

    assert!(matches!(&error, ApplicationError::Conflict(text) if text.contains("Ana")));
    assert_eq!(office.recorder.entries().len(), 0);
    assert_eq!(office.terminals.stopped().len(), 0);
    assert_eq!(office.sleep_gate.calls().len(), 0);
}

#[test]
fn a_reason_of_the_gate_conflicts_without_a_record_or_a_stop() {
    let office = Office::new();
    let (agent, _) = waiting_agent(&office);
    let entries_before = office.recorder.entries().len();
    office.sleep_gate.block_with("a hold covers the agent");

    let error = office
        .sleep_agent()
        .handle(agent.id, 30)
        .expect_err("blocked");

    assert_eq!(
        error,
        ApplicationError::Conflict("a hold covers the agent".to_owned())
    );
    assert_eq!(office.recorder.entries().len(), entries_before);
    assert_eq!(office.terminals.stopped().len(), 0);
    assert_eq!(
        office.agents.status_of(agent.id),
        Some(AgentStatus::WaitingForInput)
    );
    assert_eq!(office.sleep_gate.calls().len(), 0);
}

#[test]
fn a_stop_that_fails_records_the_failure_and_keeps_the_status() {
    let office = Office::new();
    let (agent, _) = waiting_agent(&office);
    office.terminals.refuse_stop_requests();

    let error = office
        .sleep_agent()
        .handle(agent.id, 30)
        .expect_err("stop fails");

    assert!(matches!(error, ApplicationError::Internal(_)));
    let entry = office.recorder.entries().pop().expect("entry");
    assert_eq!(entry.action, AgentAction::Sleep);
    assert_eq!(entry.failure, Some(error.to_string()));
    assert_eq!(
        office.agents.status_of(agent.id),
        Some(AgentStatus::WaitingForInput)
    );
    assert_eq!(office.sleep_gate.calls(), ["begin", "end"]);
}

#[test]
fn a_record_that_cannot_be_written_still_ends_the_gate_scope() {
    let office = Office::new();
    let (agent, _) = waiting_agent(&office);
    office.recorder.fail_from_now();

    let result = office.sleep_agent().handle(agent.id, 30);

    assert!(result.is_err());
    assert_eq!(office.sleep_gate.calls(), ["begin", "end"]);
}

#[test]
fn a_child_that_ended_on_its_own_records_a_failed_sleep_and_leaves_the_status_to_its_exit_event() {
    let office = Office::new();
    let (agent, _) = waiting_agent(&office);
    let entries_before = office.recorder.entries().len();
    office.terminals.end_children_on_their_own();

    let error = office
        .sleep_agent()
        .handle(agent.id, 30)
        .expect_err("not asleep");

    assert_eq!(
        error,
        ApplicationError::Conflict("the CLI had already ended".to_owned())
    );
    let entries = office.recorder.entries();
    assert_eq!(entries.len(), entries_before + 1);
    let entry = entries.last().expect("entry");
    assert_eq!(entry.action, AgentAction::Sleep);
    assert_eq!(entry.failure, Some(error.to_string()));
    assert_eq!(
        office.agents.status_of(agent.id),
        Some(AgentStatus::WaitingForInput)
    );
    assert_eq!(office.sleep_gate.calls(), ["begin", "end"]);
}

#[test]
fn an_unknown_agent_is_not_found() {
    let office = Office::new();

    let error = office
        .sleep_agent()
        .handle(AgentId::generate(), 30)
        .expect_err("unknown");

    assert!(matches!(error, ApplicationError::NotFound(_)));
}
