// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::ExitResult;
use tempfile::TempDir;

use super::*;
use crate::test_support::Office;

#[test]
fn sessions_come_newest_first_and_an_ended_one_with_an_id_can_continue() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);
    let first = office
        .start_agent()
        .handle(agent.id)
        .expect("start")
        .session;
    office
        .end_session()
        .handle(agent.id, &ExitResult::Code(0))
        .expect("end");
    office
        .agents
        .update_status(agent.id, AgentStatus::Exited)
        .expect("status");
    let second = office
        .start_new_task()
        .handle(agent.id)
        .expect("new task")
        .session;

    let views = office.list_sessions().handle(agent.id).expect("list");

    assert_eq!(views.len(), 2);
    assert_eq!(views[0].session.id, second.id);
    assert_eq!(views[1].session.id, first.id);
    assert!(views[1].can_continue());
    assert_eq!(
        views[0].cannot_continue_reason.as_deref(),
        Some("the session is still running")
    );
}

#[test]
fn a_generic_command_never_can_continue_and_says_why() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "generic", folder.path(), AgentStatus::Stopped);
    office.start_agent().handle(agent.id).expect("start");
    office
        .end_session()
        .handle(agent.id, &ExitResult::Code(0))
        .expect("end");

    let views = office.list_sessions().handle(agent.id).expect("list");

    assert!(!views[0].can_continue());
    assert!(
        views[0]
            .cannot_continue_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("does not support"))
    );
}

#[test]
fn an_unmatched_id_and_a_session_that_never_started_each_give_a_reason() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "tracked", folder.path(), AgentStatus::Stopped);
    let mut unmatched = office
        .start_agent()
        .handle(agent.id)
        .expect("start")
        .session;
    unmatched.cli_session_id = None;
    unmatched.end(ExitResult::Code(0), kernel::Timestamp::from_unix_millis(5));
    let mut never_started = unmatched.clone();
    never_started.id = agent_domain::value_objects::SessionId::generate();
    never_started.exit_result = Some(ExitResult::SpawnError("x".to_owned()));

    let adapters = crate::test_support::FakeAdapters;
    let tracked = adapters.for_program("tracked");

    assert!(
        continuation_of(&unmatched, &*tracked)
            .expect_err("unmatched")
            .contains("does not guess")
    );
    assert_eq!(
        continuation_of(&never_started, &*tracked).expect_err("never started"),
        "the session never started"
    );
}

#[test]
fn a_lost_session_can_be_continued() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "tracked", folder.path(), AgentStatus::Stopped);
    let mut lost = office
        .start_agent()
        .handle(agent.id)
        .expect("start")
        .session;
    lost.cli_session_id = Some("cli-1".to_owned());
    lost.end(ExitResult::Lost, kernel::Timestamp::from_unix_millis(5));
    let adapters = crate::test_support::FakeAdapters;
    let tracked = adapters.for_program("tracked");

    assert_eq!(continuation_of(&lost, &*tracked), Ok("cli-1"));
}
