// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::entities::Agent;
use agent_domain::value_objects::OfficeAdditions;
use kernel::{Actor, Timestamp};
use tempfile::TempDir;

use super::*;
use crate::common::actor_scope::ActorScope;
use crate::test_support::Office;

/// An agent whose first session ended with `result`, in `status`.
fn agent_after(
    office: &Office,
    command: &str,
    result: &ExitResult,
    status: AgentStatus,
) -> (Agent, Session) {
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent(
        "Ana",
        command,
        folder.keep().as_path(),
        AgentStatus::Stopped,
    );
    let session = office
        .start_agent()
        .handle(agent.id)
        .expect("start")
        .session;
    office.end_session().handle(agent.id, result).expect("end");
    office
        .agents
        .update_status(agent.id, status)
        .expect("status");
    (agent, session)
}

fn last_entry(office: &Office) -> crate::common::models::AgentActionEntry {
    office.recorder.entries().pop().expect("an entry")
}

#[test]
fn an_interrupted_agent_continues_its_last_session_and_the_record_has_no_detail() {
    let office = Office::new();
    let (agent, first) = agent_after(
        &office,
        "claude",
        &ExitResult::Interrupted,
        AgentStatus::Interrupted,
    );

    let outcome = office.resume_agent().resume(agent.id).expect("resume");

    assert!(outcome.continued);
    assert_eq!(outcome.reason, None);
    assert_eq!(outcome.start.session.continued_from, Some(first.id));
    let entry = last_entry(&office);
    assert_eq!(entry.action, AgentAction::Continue);
    assert_eq!(entry.detail, None);
}

#[test]
fn a_lost_session_is_continued_like_an_interrupted_one() {
    let office = Office::new();
    let (agent, first) = agent_after(
        &office,
        "claude",
        &ExitResult::Lost,
        AgentStatus::Interrupted,
    );

    let outcome = office.resume_agent().resume(agent.id).expect("resume");

    assert!(outcome.continued);
    assert_eq!(outcome.start.session.continued_from, Some(first.id));
}

#[test]
fn a_cli_that_cannot_continue_gets_a_new_session_with_the_reason_in_the_record() {
    let office = Office::new();
    let (agent, _) = agent_after(
        &office,
        "generic",
        &ExitResult::Interrupted,
        AgentStatus::Interrupted,
    );

    let outcome = office.resume_agent().resume(agent.id).expect("resume");

    let reason = "the agent's command does not support continuing a session";
    assert!(!outcome.continued);
    assert_eq!(outcome.reason.as_deref(), Some(reason));
    assert_eq!(outcome.start.session.continued_from, None);
    let entry = last_entry(&office);
    assert_eq!(entry.action, AgentAction::Start);
    assert_eq!(
        entry.detail.as_deref(),
        Some(format!("new session: {reason}").as_str())
    );
}

#[test]
fn an_agent_without_a_session_gets_a_new_one() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Interrupted);

    let outcome = office.resume_agent().resume(agent.id).expect("resume");

    assert!(!outcome.continued);
    assert_eq!(
        last_entry(&office).detail.as_deref(),
        Some("new session: the agent has no earlier session")
    );
}

#[test]
fn an_agent_that_is_not_interrupted_is_a_conflict_and_nothing_starts() {
    let office = Office::new();
    let (agent, _) = agent_after(&office, "claude", &ExitResult::Code(0), AgentStatus::Exited);

    let error = office
        .resume_agent()
        .resume(agent.id)
        .expect_err("conflict");

    assert_eq!(
        error,
        ApplicationError::Conflict("the agent is no longer interrupted".to_owned())
    );
    assert_eq!(office.terminals.started().len(), 1);
}

#[test]
fn an_unknown_agent_is_not_found() {
    let office = Office::new();

    let error = office
        .resume_agent()
        .resume(AgentId::generate())
        .expect_err("unknown");

    assert!(matches!(error, ApplicationError::NotFound(_)));
}

#[test]
fn a_command_that_cannot_start_is_a_success_with_a_failure_that_the_record_keeps() {
    let office = Office::new();
    let (agent, _) = agent_after(
        &office,
        "nonesuch",
        &ExitResult::Interrupted,
        AgentStatus::Interrupted,
    );
    office.terminals.make_unstartable("nonesuch");

    let outcome = office.resume_agent().resume(agent.id).expect("resume");

    assert!(outcome.start.failure.is_some());
    assert_eq!(outcome.start.agent.status, AgentStatus::Failed);
    assert_eq!(last_entry(&office).failure, outcome.start.failure);
}

#[test]
fn the_fall_back_starts_a_new_session_after_a_continue_that_ended_by_itself() {
    let office = Office::new();
    let (agent, refused) =
        agent_after(&office, "claude", &ExitResult::Code(1), AgentStatus::Exited);

    let _scope = ActorScope::enter(Actor::Office);
    let outcome = office
        .resume_agent()
        .fall_back(agent.id, refused.id)
        .expect("fall back");

    assert_eq!(outcome.start.session.continued_from, None);
    assert!(!outcome.continued);
    assert_eq!(
        outcome.reason.as_deref(),
        Some("the CLI ended 0 s after the continue, with exit code 1")
    );
    let entry = last_entry(&office);
    assert_eq!(entry.actor, Actor::Office);
    assert_eq!(entry.action, AgentAction::Start);
    assert_eq!(
        entry.detail.as_deref(),
        Some("new session: the CLI ended 0 s after the continue, with exit code 1")
    );
}

#[test]
fn the_fall_back_is_a_conflict_after_a_stop() {
    let office = Office::new();
    let (agent, refused) =
        agent_after(&office, "claude", &ExitResult::Signal, AgentStatus::Stopped);

    let error = office
        .resume_agent()
        .fall_back(agent.id, refused.id)
        .expect_err("conflict");

    assert_eq!(
        error,
        ApplicationError::Conflict("the agent changed after the refused session".to_owned())
    );
    assert_eq!(office.terminals.started().len(), 1);
}

#[test]
fn the_fall_back_is_a_conflict_when_a_newer_session_exists() {
    let office = Office::new();
    let (agent, refused) =
        agent_after(&office, "claude", &ExitResult::Code(1), AgentStatus::Failed);
    office.start_agent().handle(agent.id).expect("newer start");
    office
        .end_session()
        .handle(agent.id, &ExitResult::Code(1))
        .expect("end");
    office
        .agents
        .update_status(agent.id, AgentStatus::Failed)
        .expect("status");

    let error = office
        .resume_agent()
        .fall_back(agent.id, refused.id)
        .expect_err("conflict");

    assert!(matches!(error, ApplicationError::Conflict(_)));
}

fn ended_session(started: i64, ended: i64, result: ExitResult) -> Session {
    let mut session = Session::begin(
        SessionId::generate(),
        AgentId::generate(),
        None,
        None,
        OfficeAdditions::default(),
        Timestamp::from_unix_millis(started),
    );
    session.end(result, Timestamp::from_unix_millis(ended));
    session
}

#[test]
fn the_reason_counts_whole_seconds_rounded_down() {
    let session = ended_session(1_000, 4_999, ExitResult::Code(2));

    assert_eq!(
        refusal_reason(&session).as_deref(),
        Some("the CLI ended 3 s after the continue, with exit code 2")
    );
}

#[test]
fn the_reason_names_a_result_that_is_not_a_code() {
    let session = ended_session(0, 2_000, ExitResult::Signal);

    assert_eq!(
        refusal_reason(&session).as_deref(),
        Some("the CLI ended 2 s after the continue, with result signal")
    );
}

#[test]
fn the_reason_of_a_continue_that_did_not_start_carries_the_text() {
    let session = ended_session(0, 0, ExitResult::SpawnError("no such program".to_owned()));

    assert_eq!(
        refusal_reason(&session).as_deref(),
        Some("the continue did not start: no such program")
    );
}
