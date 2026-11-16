// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::enums::AgentStatus;
use tempfile::TempDir;

use super::*;
use crate::test_support::Office;

#[test]
fn the_running_session_gets_the_exit_code_once() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);
    let session = office
        .start_agent()
        .handle(agent.id)
        .expect("start")
        .session;

    let first = office
        .end_session()
        .handle(agent.id, &ExitResult::Code(3))
        .expect("end");
    let second = office
        .end_session()
        .handle(agent.id, &ExitResult::Code(9))
        .expect("again");

    assert!(first);
    assert!(!second, "a second end changes nothing");
    let stored = office
        .sessions
        .find(session.id)
        .expect("find")
        .expect("stored");
    assert_eq!(stored.exit_result, Some(ExitResult::Code(3)));
    assert!(stored.ended_at.is_some());
}
