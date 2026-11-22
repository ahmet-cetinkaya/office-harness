// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::TASK_TEXT_MAX_CHARS;
use kernel::{Actor, Timestamp};
use tempfile::TempDir;

use super::*;
use crate::test_support::Office;

fn stored_task(office: &Office, agent_id: AgentId) -> Option<TaskText> {
    office
        .agents
        .find(agent_id)
        .expect("find")
        .expect("agent")
        .task
}

fn add_agent(office: &Office) -> Agent {
    let folder = TempDir::new().expect("folder");
    office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped)
}

#[test]
fn a_text_of_the_limit_is_stored_with_the_user_as_source() {
    let office = Office::new();
    let agent = add_agent(&office);
    let text = "x".repeat(TASK_TEXT_MAX_CHARS);

    let updated = office
        .set_task_text()
        .handle(agent.id, &text)
        .expect("accepted");

    let task = stored_task(&office, agent.id).expect("task");
    assert_eq!(task.text(), text);
    assert_eq!(task.source(), TaskSource::User);
    assert_eq!(
        task.set_at(),
        Timestamp::from_unix_millis(1_700_000_000_000)
    );
    assert_eq!(updated.task, Some(task));
}

#[test]
fn a_text_past_the_limit_is_refused_with_the_message_and_changes_nothing() {
    let office = Office::new();
    let agent = add_agent(&office);

    let error = office
        .set_task_text()
        .handle(agent.id, &"x".repeat(TASK_TEXT_MAX_CHARS + 1))
        .expect_err("too long");

    assert_eq!(
        error,
        ApplicationError::Invalid("a task text has at most 8,192 characters".to_owned())
    );
    assert_eq!(stored_task(&office, agent.id), None);
    assert_eq!(office.recorder.entries().len(), 0);
}

#[test]
fn a_blank_text_clears_the_task() {
    let office = Office::new();
    let agent = add_agent(&office);
    office
        .set_task_text()
        .handle(agent.id, "Review PR 12")
        .expect("set");

    let updated = office
        .set_task_text()
        .handle(agent.id, &" ".repeat(TASK_TEXT_MAX_CHARS + 1))
        .expect("clear");

    assert_eq!(updated.task, None);
    assert_eq!(stored_task(&office, agent.id), None);
}

#[test]
fn control_characters_are_stored_unchanged() {
    let office = Office::new();
    let agent = add_agent(&office);
    let text = "\u{1b}[31mred\nline two\u{7}";

    office.set_task_text().handle(agent.id, text).expect("set");

    assert_eq!(stored_task(&office, agent.id).expect("task").text(), text);
}

#[test]
fn a_change_adds_one_task_text_entry_with_the_user_as_actor() {
    let office = Office::new();
    let agent = add_agent(&office);

    office
        .set_task_text()
        .handle(agent.id, "Review PR 12")
        .expect("set");

    let entries = office.recorder.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, AgentAction::TaskText);
    assert_eq!(entries[0].actor, Actor::User);
    assert_eq!(entries[0].agent_id, agent.id);
    assert_eq!(entries[0].failure, None);
}

#[test]
fn an_unknown_agent_is_not_found() {
    let office = Office::new();

    let error = office
        .set_task_text()
        .handle(AgentId::generate(), "Review PR 12")
        .expect_err("unknown");

    assert!(matches!(error, ApplicationError::NotFound(_)), "{error:?}");
}
