// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::enums::AgentStatus;
use tempfile::TempDir;

use super::*;
use crate::test_support::Office;

fn ended_session(office: &Office, name: &str, command: &str) -> (Agent, Session) {
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent(name, command, folder.keep().as_path(), AgentStatus::Stopped);
    let session = office
        .start_agent()
        .handle(agent.id)
        .expect("start")
        .session;
    office
        .end_session()
        .handle(agent.id, &agent_domain::value_objects::ExitResult::Code(0))
        .expect("end");
    office
        .agents
        .update_status(agent.id, AgentStatus::Exited)
        .expect("status");
    (agent, session)
}

#[test]
fn continuing_starts_the_cli_with_the_old_id_and_records_where_it_came_from() {
    let office = Office::new();
    let (agent, first) = ended_session(&office, "Ana", "claude");
    let cli_id = first.cli_session_id.clone().expect("id");

    let outcome = office
        .continue_session()
        .handle(first.id)
        .expect("continue");

    assert_eq!(outcome.session.continued_from, Some(first.id));
    assert_eq!(
        outcome.session.cli_session_id.as_deref(),
        Some(cli_id.as_str())
    );
    let started = office.terminals.started();
    assert_eq!(started.len(), 2);
    assert_eq!(started[1].arguments, ["--resume", cli_id.as_str()]);
    assert_eq!(office.sessions.all().len(), 2);
    assert_eq!(
        office.recorder.entries().last().map(|entry| entry.action),
        Some(AgentAction::Continue)
    );
    assert_eq!(outcome.agent.id, agent.id);
}

#[test]
fn the_startup_prompt_goes_only_to_a_new_session_and_not_to_a_continue() {
    let office = Office::new();
    let (agent, first) = ended_session(&office, "Ana", "claude");
    office
        .set_additions()
        .handle(
            agent.id,
            crate::common::additions_input::AdditionsInput {
                startup_prompt: Some("only once".to_owned()),
                ..Default::default()
            },
        )
        .expect("additions");

    let outcome = office
        .continue_session()
        .handle(first.id)
        .expect("continue");

    assert!(
        !office.terminals.started()[1]
            .arguments
            .contains(&"only once".to_owned())
    );
    assert_eq!(outcome.session.additions_snapshot.startup_prompt(), None);
}

#[test]
fn a_cli_that_cannot_continue_is_refused_with_a_reason_and_starts_nothing() {
    let office = Office::new();
    let (_, first) = ended_session(&office, "Ana", "generic");

    let error = office
        .continue_session()
        .handle(first.id)
        .expect_err("refused");

    assert!(matches!(&error, ApplicationError::Refused(text) if text.contains("does not support")));
    assert_eq!(office.terminals.started().len(), 1);
}

#[test]
fn a_session_without_a_matched_id_is_refused_and_never_guessed() {
    let office = Office::new();
    // `tracked` learns its id only after the start, so a stored session
    // that was never matched has none.
    let (_, first) = ended_session(&office, "Ana", "tracked");
    let stored = office.sessions.all();
    assert_eq!(stored.len(), 1);
    let mut unmatched = stored[0].clone();
    unmatched.id = agent_domain::value_objects::SessionId::generate();
    unmatched.cli_session_id = None;
    office.sessions.insert(&unmatched).expect("insert");

    let error = office
        .continue_session()
        .handle(unmatched.id)
        .expect_err("refused");

    assert!(matches!(&error, ApplicationError::Refused(text) if text.contains("does not guess")));
    assert_eq!(office.terminals.started().len(), 1);
    assert!(first.cli_session_id.is_none());
}

#[test]
fn an_unknown_session_is_not_found() {
    let office = Office::new();

    let error = office
        .continue_session()
        .handle(agent_domain::value_objects::SessionId::generate())
        .expect_err("unknown");

    assert!(matches!(error, ApplicationError::NotFound(_)));
    assert_eq!(office.recorder.entries().len(), 0);
}

#[test]
fn a_continue_that_is_refused_leaves_the_running_session_running() {
    let office = Office::new();
    let (agent, first) = ended_session(&office, "Ana", "claude");
    office
        .agents
        .update_status(agent.id, AgentStatus::Working)
        .expect("status");
    let folder = agent.working_folder.clone().expect("folder");
    std::fs::remove_dir_all(folder).expect("remove");

    let error = office
        .continue_session()
        .handle(first.id)
        .expect_err("the folder is gone");

    assert!(matches!(error, ApplicationError::Invalid(_)), "{error:?}");
    assert_eq!(office.terminals.stopped().len(), 0);
    assert_eq!(
        office.agents.status_of(agent.id),
        Some(AgentStatus::Working)
    );
}
