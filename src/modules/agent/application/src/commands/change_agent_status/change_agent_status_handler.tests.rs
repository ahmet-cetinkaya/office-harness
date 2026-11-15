// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::enums::AgentStatus;
use tempfile::TempDir;

use super::*;
use crate::test_support::Office;

fn status_after(office: &Office, agent_id: AgentId, events: &[StatusEvent]) -> Option<AgentStatus> {
    let handler = ChangeAgentStatusHandler::new(office.agents.clone());
    for event in events {
        handler.handle(agent_id, *event).expect("change");
    }
    office.agents.status_of(agent_id)
}

#[test]
fn the_status_follows_starting_working_waiting_and_exited() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Starting);
    let handler = ChangeAgentStatusHandler::new(office.agents.clone());

    let steps = [
        (StatusEvent::FirstOutput, AgentStatus::Working),
        (
            StatusEvent::QuietPeriodElapsed,
            AgentStatus::WaitingForInput,
        ),
        (StatusEvent::Output, AgentStatus::Working),
        (StatusEvent::ExitedWithCodeZero, AgentStatus::Exited),
    ];
    for (event, expected) in steps {
        let changed = handler
            .handle(agent.id, event)
            .expect("change")
            .expect("moved");
        assert_eq!(changed.status, expected);
        assert_eq!(office.agents.status_of(agent.id), Some(expected));
    }
}

#[test]
fn a_failed_exit_makes_the_agent_failed() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Working);

    let status = status_after(&office, agent.id, &[StatusEvent::ExitedWithFailure]);

    assert_eq!(status, Some(AgentStatus::Failed));
}

#[test]
fn an_exit_after_a_user_stop_changes_nothing() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);
    let handler = ChangeAgentStatusHandler::new(office.agents.clone());

    let changed = handler
        .handle(agent.id, StatusEvent::ExitedWithFailure)
        .expect("ignored");

    assert_eq!(changed, None);
    assert_eq!(
        office.agents.status_of(agent.id),
        Some(AgentStatus::Stopped)
    );
}

#[test]
fn an_event_for_a_deleted_agent_changes_nothing() {
    let office = Office::new();
    let handler = ChangeAgentStatusHandler::new(office.agents.clone());

    let changed = handler
        .handle(AgentId::generate(), StatusEvent::Output)
        .expect("ignored");

    assert_eq!(changed, None);
}
