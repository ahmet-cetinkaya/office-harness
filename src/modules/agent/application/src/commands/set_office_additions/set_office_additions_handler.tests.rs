// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::enums::AgentStatus;
use tempfile::TempDir;

use super::*;
use crate::test_support::Office;

fn input(name: &str) -> AdditionsInput {
    AdditionsInput {
        flags: vec!["--x".to_owned()],
        env: vec![(name.to_owned(), "v".to_owned())],
        startup_prompt: None,
    }
}

#[test]
fn a_malformed_environment_name_is_refused_and_named_and_nothing_is_stored() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);

    let error = office
        .set_additions()
        .handle(agent.id, input("BAD NAME"))
        .expect_err("refused");

    assert!(matches!(&error, ApplicationError::Invalid(text) if text.contains("\"BAD NAME\"")));
    assert_eq!(office.additions.load(agent.id).expect("load"), None);
}

#[test]
fn an_edit_on_a_running_agent_says_it_applies_at_the_next_session() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let running = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Working);
    let stopped = office.add_agent("Ben", "claude", folder.path(), AgentStatus::Stopped);

    let on_running = office
        .set_additions()
        .handle(running.id, input("A"))
        .expect("set");
    let on_stopped = office
        .set_additions()
        .handle(stopped.id, input("B"))
        .expect("set");

    assert!(on_running.applies_next_session);
    assert!(!on_stopped.applies_next_session);
}

#[test]
fn an_unknown_agent_is_not_found() {
    let office = Office::new();

    let error = office
        .set_additions()
        .handle(AgentId::generate(), input("A"))
        .expect_err("unknown");

    assert!(matches!(error, ApplicationError::NotFound(_)));
}
