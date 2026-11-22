// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::common::errors::PortError;
use crate::common::models::AgentActionEntry;
use crate::test_support::{Office, absolute_folder, project_named};

struct FailingAdditions;

impl AdditionsStore for FailingAdditions {
    fn load(&self, _agent_id: AgentId) -> Result<Option<AdditionsRecord>, PortError> {
        Ok(None)
    }

    fn save(&self, _agent_id: AgentId, _record: &AdditionsRecord) -> Result<(), PortError> {
        Err(PortError::Storage {
            message: "disk full".to_owned(),
            is_disk_failure: true,
        })
    }
}

fn request(name: &str) -> CreateAgentRequest {
    CreateAgentRequest {
        name: name.to_owned(),
        command: "claude".to_owned(),
        working_folder: Some(absolute_folder("work")),
        project_id: None,
        additions: None,
    }
}

#[test]
fn a_new_agent_shows_stopped_and_adds_one_create_entry() {
    let office = Office::new();

    let agent = office
        .create_agent()
        .handle(request("Ana"))
        .expect("create");

    assert_eq!(agent.status, agent_domain::enums::AgentStatus::Stopped);
    assert_eq!(office.agents.all(), std::slice::from_ref(&agent));
    let entries: Vec<AgentActionEntry> = office.recorder.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, AgentAction::Create);
    assert_eq!(entries[0].agent_id, agent.id);
    assert_eq!(entries[0].actor, kernel::Actor::User);
    assert_eq!(entries[0].failure, None);
}

#[test]
fn a_name_that_differs_only_in_case_conflicts_and_adds_no_entry() {
    let office = Office::new();
    office.create_agent().handle(request("Ana")).expect("first");

    let error = office
        .create_agent()
        .handle(request("ANA"))
        .expect_err("duplicate");

    assert!(matches!(&error, ApplicationError::Conflict(text) if text.contains("agent name")));
    assert_eq!(office.recorder.entries().len(), 1);
}

#[test]
fn a_blank_command_is_invalid_and_names_the_field() {
    let office = Office::new();
    let mut blank = request("Ana");
    blank.command = " ".to_owned();

    let error = office.create_agent().handle(blank).expect_err("blank");

    assert!(matches!(&error, ApplicationError::Invalid(text) if text.contains("command")));
}

#[test]
fn an_agent_with_neither_folder_nor_project_is_invalid() {
    let office = Office::new();
    let mut bare = request("Ana");
    bare.working_folder = Some(PathBuf::new());

    let error = office.create_agent().handle(bare).expect_err("no folder");

    assert!(matches!(&error, ApplicationError::Invalid(text) if text.contains("working folder")));
}

#[test]
fn a_relative_working_folder_is_invalid_and_names_the_field() {
    let office = Office::new();
    let mut relative = request("Ana");
    relative.working_folder = Some(PathBuf::from("work"));

    let error = office
        .create_agent()
        .handle(relative)
        .expect_err("relative");

    assert!(matches!(&error, ApplicationError::Invalid(text) if text.contains("working folder")));
}

#[test]
fn an_unknown_project_is_not_found() {
    let office = Office::new();
    let mut orphan = request("Ana");
    orphan.project_id = Some(ProjectId::generate());

    let error = office.create_agent().handle(orphan).expect_err("unknown");

    assert!(matches!(error, ApplicationError::NotFound(_)));
}

#[test]
fn a_project_replaces_the_working_folder() {
    let office = Office::new();
    let project = project_named("Docs");
    office.projects.insert(&project).expect("project");
    let mut in_project = request("Ana");
    in_project.working_folder = None;
    in_project.project_id = Some(project.id);

    let agent = office.create_agent().handle(in_project).expect("create");

    assert_eq!(agent.project_id, Some(project.id));
    assert_eq!(agent.working_folder, None);
}

#[test]
fn a_command_with_a_quote_that_is_never_closed_is_invalid() {
    let office = Office::new();
    let mut open_quote = request("Ana");
    open_quote.command = r#"claude "oops"#.to_owned();

    let error = office
        .create_agent()
        .handle(open_quote)
        .expect_err("open quote");

    assert!(matches!(&error, ApplicationError::Invalid(text) if text.contains("command")));
    assert_eq!(office.agents.all().len(), 0);
}

#[test]
fn a_malformed_environment_name_is_refused_and_named_and_nothing_is_created() {
    let office = Office::new();
    let mut with_bad_entry = request("Ana");
    with_bad_entry.additions = Some(AdditionsInput {
        env: vec![("1BAD".to_owned(), Some("x".to_owned()))],
        ..AdditionsInput::default()
    });

    let error = office
        .create_agent()
        .handle(with_bad_entry)
        .expect_err("malformed");

    assert!(matches!(&error, ApplicationError::Invalid(text) if text.contains("\"1BAD\"")));
    assert_eq!(office.agents.all().len(), 0);
    assert_eq!(office.recorder.entries().len(), 0);
}

#[test]
fn the_additions_given_at_creation_are_stored_with_the_agent() {
    let office = Office::new();
    let mut with_additions = request("Ana");
    with_additions.additions = Some(AdditionsInput {
        flags: vec!["--verbose".to_owned()],
        env: vec![("KEY".to_owned(), Some("value".to_owned()))],
        startup_prompt: Some("hello".to_owned()),
    });

    let agent = office
        .create_agent()
        .handle(with_additions)
        .expect("create");

    let view = office.get_additions().handle(agent.id).expect("additions");
    assert_eq!(view.record.additions.flags(), ["--verbose"]);
    assert_eq!(view.record.additions.env()[0].name(), "KEY");
    assert_eq!(view.record.additions.startup_prompt(), Some("hello"));
}

#[test]
fn an_agent_whose_additions_cannot_be_saved_is_not_left_behind() {
    let office = Office::new();
    let handler = CreateAgentHandler::new(
        office.agents.clone(),
        office.projects.clone(),
        Arc::new(FailingAdditions),
        Office::clock(),
        ActionJournal::new(office.recorder.clone(), Office::clock()),
        office.machine_id,
    );

    let error = handler.handle(request("Ana")).expect_err("save fails");

    assert!(
        matches!(error, ApplicationError::Storage { .. }),
        "{error:?}"
    );
    assert_eq!(office.agents.all().len(), 0);
    assert_eq!(office.recorder.entries().len(), 0);
}
