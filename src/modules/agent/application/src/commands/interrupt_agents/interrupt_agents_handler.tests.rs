// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Instant;

use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::Settings;
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

#[test]
fn agents_that_ignore_the_stop_cost_one_grace_period_together_and_not_one_each() {
    const AGENTS: usize = 5;
    // Serial stops would take five graces; together they take about one.
    const GRACES_AT_MOST: u32 = 3;
    const GRACE_MS: u64 = 200;
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    office
        .settings
        .save(&Settings {
            stop_grace_ms: GRACE_MS,
            ..Settings::default()
        })
        .expect("settings");
    for number in 0..AGENTS {
        let agent = office.add_agent(
            &format!("Agent {number}"),
            "claude",
            folder.path(),
            AgentStatus::Stopped,
        );
        office.start_agent().handle(agent.id).expect("start");
    }
    office.terminals.ignore_stop_requests();

    let started = Instant::now();
    let outcome = interrupter(&office).handle().expect("interrupt");
    let elapsed = started.elapsed();

    assert_eq!(outcome.interrupted.len(), AGENTS);
    assert_eq!(outcome.failures, Vec::<String>::new());
    assert_eq!(office.terminals.stopped().len(), AGENTS);
    assert!(
        elapsed < Duration::from_millis(GRACE_MS) * GRACES_AT_MOST,
        "the stops took {elapsed:?}"
    );
}
