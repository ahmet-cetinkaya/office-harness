// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::enums::AgentStatus;
use tempfile::TempDir;

use super::*;
use crate::test_support::Office;

#[test]
fn an_agent_without_stored_additions_shows_empty_ones() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);

    let view = office.get_additions().handle(agent.id).expect("get");

    assert_eq!(view.record.additions, OfficeAdditions::default());
    assert!(!view.applies_next_session);
}

#[test]
fn an_unknown_agent_is_not_found() {
    let office = Office::new();

    let error = office
        .get_additions()
        .handle(AgentId::generate())
        .expect_err("unknown");

    assert!(matches!(error, ApplicationError::NotFound(_)));
}
