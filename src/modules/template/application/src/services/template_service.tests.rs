// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::json;
use template_domain::{EnvVariable, MemberEntry, TeamBody, TemplateError, TemplateRole};

use super::*;
use crate::test_support::{agent_body, agent_template_body, service};

fn team_body() -> TemplateBody {
    TemplateBody::Team(TeamBody {
        lead: agent_body("{team}-lead"),
        members: vec![MemberEntry {
            entry: agent_body("{team}-member-{n}"),
            count: 2,
        }],
    })
}

fn file_of(names: &[&str]) -> String {
    let templates: Vec<_> = names
        .iter()
        .map(|name| {
            json!({
                "name": name, "kind": "agent",
                "body": {"body_version": 1, "name_pattern": "x-{n}", "command": "claude",
                         "additions": {"env": [{"name": "KEY", "value": "crafted"}]}}
            })
        })
        .collect();
    json!({"format": "office-harness-templates", "version": 1, "templates": templates}).to_string()
}

#[test]
fn a_saved_template_is_listed_and_fetched() {
    let service = service();

    let saved = service
        .save(None, " Reviewer ", agent_template_body("r-{n}"))
        .expect("save");

    assert_eq!(saved.name.as_str(), "Reviewer");
    assert_eq!(service.list().expect("list"), std::slice::from_ref(&saved));
    assert_eq!(service.get(saved.id).expect("get"), saved);
}

#[test]
fn a_name_used_ignoring_case_and_a_broken_rule_are_refused() {
    let service = service();
    service
        .save(None, "Reviewer", agent_template_body("r-{n}"))
        .expect("save");

    let taken = service
        .save(None, "REVIEWER", agent_template_body("r-{n}"))
        .expect_err("taken");
    let mut empty = agent_body("r-{n}");
    empty.command = String::new();
    let broken = service
        .save(None, "Other", TemplateBody::Agent(empty))
        .expect_err("rule");

    assert!(matches!(taken, TemplateApplicationError::NameTaken { .. }));
    assert!(matches!(
        broken,
        TemplateApplicationError::Invalid(TemplateError::CommandEmpty { .. })
    ));
}

#[test]
fn saving_with_an_id_replaces_and_an_unknown_id_is_not_found() {
    let service = service();
    let saved = service
        .save(None, "Reviewer", agent_template_body("r-{n}"))
        .expect("save");

    let replaced = service
        .save(
            Some(saved.id),
            "Reviewer",
            agent_template_body("changed-{n}"),
        )
        .expect("replace");
    let missing = service
        .save(
            Some(TemplateId::generate()),
            "Ghost",
            agent_template_body("g"),
        )
        .expect_err("missing");

    assert_eq!(service.list().expect("list"), [replaced]);
    assert!(matches!(missing, TemplateApplicationError::NotFound(_)));
}

#[test]
fn deleting_a_template_removes_it_once() {
    let service = service();
    let saved = service
        .save(None, "Reviewer", agent_template_body("r-{n}"))
        .expect("save");

    service.delete(saved.id).expect("delete");

    assert_eq!(service.list().expect("list"), []);
    assert!(matches!(
        service.delete(saved.id),
        Err(TemplateApplicationError::NotFound(_))
    ));
}

#[test]
fn a_team_template_keeps_its_copy_when_an_agent_template_is_edited_or_deleted() {
    let service = service();
    let agent = service
        .save(None, "Member", agent_template_body("{team}-member-{n}"))
        .expect("agent");
    let team = service.save(None, "Squad", team_body()).expect("team");

    service
        .save(Some(agent.id), "Member", agent_template_body("changed"))
        .expect("edit");
    service.delete(agent.id).expect("delete");

    assert_eq!(service.get(team.id).expect("team"), team);
}

#[test]
fn a_body_copied_from_an_agent_gets_the_default_pattern_and_the_agents_values() {
    let service = service();

    let body = service.body_from_agent(AgentId::generate()).expect("body");

    assert_eq!(body.name_pattern.as_str(), "Ana-{n}");
    assert_eq!(body.command, "claude --model opus");
    assert_eq!(body.working_folder.as_deref(), Some("/work"));
    assert_eq!(body.role, None::<TemplateRole>);
    assert_eq!(service.list().expect("list"), []);
}

#[test]
fn an_export_holds_no_environment_value_and_keeps_the_names() {
    let service = service();
    let saved = service
        .save(None, "Reviewer", agent_template_body("r-{n}"))
        .expect("save");

    let text = service.export(None).expect("export");

    assert!(!text.contains("secret"));
    assert!(text.contains("API_KEY"));
    assert_eq!(service.export(Some(&[saved.id])).expect("chosen"), text);
    assert!(matches!(
        service.export(Some(&[TemplateId::generate()])),
        Err(TemplateApplicationError::NotFound(_))
    ));
    assert_eq!(
        service.get(saved.id).expect("get"),
        saved,
        "the office keeps its values"
    );
}

#[test]
fn a_preview_names_the_save_names_drops_values_and_saves_nothing() {
    let service = service();
    service
        .save(None, "Reviewer", agent_template_body("r-{n}"))
        .expect("save");

    let preview = service
        .preview_import(&file_of(&["reviewer", "Reviewer", "Fresh"]))
        .expect("preview");

    let names: Vec<_> = preview.iter().map(|e| e.save_name.to_string()).collect();
    assert_eq!(names, ["reviewer (2)", "Reviewer (3)", "Fresh"]);
    let TemplateBody::Agent(agent) = &preview[0].body else {
        panic!("not an agent");
    };
    assert_eq!(
        agent.additions.env,
        [EnvVariable {
            name: "KEY".to_owned(),
            value: None
        }]
    );
    assert_eq!(service.list().expect("list").len(), 1);
}

#[test]
fn a_confirm_saves_every_template_without_values() {
    let service = service();

    let saved = service
        .confirm_import(&file_of(&["One", "Two"]))
        .expect("confirm");

    assert_eq!(saved.len(), 2);
    for template in service.list().expect("list") {
        let TemplateBody::Agent(agent) = template.body else {
            panic!("not an agent");
        };
        assert_eq!(agent.additions.env[0].value, None);
    }
}

#[test]
fn a_refused_file_saves_nothing_and_names_the_template_and_the_reason() {
    let service = service();
    let mut file: serde_json::Value =
        serde_json::from_str(&file_of(&["Good", "Bad"])).expect("json");
    file["templates"][1]["body"]["command"] = json!("");

    let error = service
        .confirm_import(&file.to_string())
        .expect_err("refused");

    let text = error.to_string();
    assert!(text.contains("template \"Bad\""), "{text}");
    assert!(text.contains("the CLI command is empty"), "{text}");
    assert_eq!(service.list().expect("list"), []);
}
