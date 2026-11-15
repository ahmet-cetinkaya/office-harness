// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::enums::AgentStatus;
use tempfile::TempDir;

use super::*;
use crate::test_support::Office;

fn interrupter(office: &Office) -> InterruptAgentsHandler {
    InterruptAgentsHandler::new(
        office.agents.clone(),
        office.settings.clone(),
        office.sessions.clone(),
        office.terminals.clone(),
        Office::clock(),
    )
}

#[test]
fn running_agents_are_stopped_and_marked_interrupted_with_their_sessions_and_others_stay() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let running = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);
    let session = office
        .start_agent()
        .handle(running.id)
        .expect("start")
        .session;
    let finished = office.add_agent("Ben", "claude", folder.path(), AgentStatus::Exited);

    let outcome = interrupter(&office).handle().expect("interrupt");

    assert_eq!(outcome.interrupted.len(), 1);
    assert_eq!(outcome.interrupted[0].id, running.id);
    assert_eq!(outcome.failures, Vec::<String>::new());
    assert_eq!(
        office.agents.status_of(running.id),
        Some(AgentStatus::Interrupted)
    );
    assert_eq!(
        office.agents.status_of(finished.id),
        Some(AgentStatus::Exited)
    );
    assert_eq!(office.terminals.stopped().len(), 1);
    let stored = office
        .sessions
        .find(session.id)
        .expect("find")
        .expect("stored");
    assert_eq!(stored.exit_result, Some(ExitResult::Interrupted));
}
