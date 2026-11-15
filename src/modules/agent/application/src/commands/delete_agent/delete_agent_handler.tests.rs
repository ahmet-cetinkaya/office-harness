// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::enums::AgentStatus;
use tempfile::TempDir;

use super::*;
use crate::test_support::Office;

#[test]
fn a_stopped_agent_is_deleted_and_an_entry_names_it() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);

    office.delete_agent().handle(agent.id).expect("delete");

    assert_eq!(office.agents.all().len(), 0);
    assert_eq!(office.terminals.stopped().len(), 0);
    let entries = office.recorder.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, AgentAction::Delete);
    assert_eq!(entries[0].agent_id, agent.id);
}

#[test]
fn a_running_agent_is_stopped_before_it_is_deleted() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Working);
    let other = office.add_agent("Ben", "claude", folder.path(), AgentStatus::Working);

    office.delete_agent().handle(agent.id).expect("delete");

    assert_eq!(office.terminals.stopped().len(), 1);
    assert_eq!(office.terminals.stopped()[0].0, agent.id);
    assert_eq!(office.agents.all(), [other]);
}

#[test]
fn an_unknown_agent_is_not_found() {
    let office = Office::new();

    let error = office
        .delete_agent()
        .handle(AgentId::generate())
        .expect_err("unknown");

    assert!(matches!(error, ApplicationError::NotFound(_)));
    assert_eq!(office.recorder.entries().len(), 0);
}
