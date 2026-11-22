// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::json;

use super::*;

fn agent_entry(name: &str) -> Value {
    json!({
        "name": name,
        "kind": "agent",
        "body": {
            "body_version": 1,
            "name_pattern": "x-{n}",
            "command": "claude",
            "additions": {"env": [{"name": "API_KEY", "value": "secret"}]}
        }
    })
}

fn team_entry(members: &Value) -> Value {
    let entry = json!({"name_pattern": "m-{n}", "command": "claude"});
    json!({
        "name": "Squad",
        "kind": "team",
        "body": {"body_version": 1, "lead": entry, "members": members}
    })
}

fn file(templates: &[Value]) -> String {
    json!({"format": "office-harness-templates", "version": 1, "templates": templates}).to_string()
}

fn refusal(text: &str) -> String {
    parse_file(text).expect_err("refused").to_string()
}

#[test]
fn a_valid_file_is_read_and_its_values_are_kept_for_the_caller_to_drop() {
    let templates = parse_file(&file(&[agent_entry("Reviewer")])).expect("file");

    assert_eq!(templates[0].name.as_str(), "Reviewer");
    let TemplateBody::Agent(agent) = &templates[0].body else {
        panic!("not an agent");
    };
    assert_eq!(agent.additions.env[0].value.as_deref(), Some("secret"));
    assert_eq!(
        templates[0].body.clone().without_env_values(),
        TemplateBody::Agent(agent.clone().without_env_values())
    );
}

#[test]
fn export_writes_every_environment_value_as_null_and_the_file_reads_back() {
    let templates = parse_file(&file(&[agent_entry("Reviewer")])).expect("file");

    let text = render_file(&templates);

    assert!(!text.contains("secret"));
    let again = parse_file(&text).expect("again");
    let TemplateBody::Agent(agent) = &again[0].body else {
        panic!("not an agent");
    };
    assert_eq!(agent.additions.env[0].value, None);
}

#[test]
fn an_unknown_version_unreadable_text_and_a_large_file_are_refused_with_a_reason() {
    let version = json!({"format": "office-harness-templates", "version": 2, "templates": []});

    assert!(refusal(&version.to_string()).contains("the file version 2 is not supported"));
    assert!(refusal("not json").contains("the file is not a template file"));
    assert!(refusal("[]").contains("the file is not a template file"));
    assert!(refusal(r#"{"format":"other","version":1}"#).contains("format is not"));
    assert!(refusal(&" ".repeat(MAX_FILE_BYTES + 1)).contains("larger than 256 KiB"));
    assert!(refusal(&file(&[])).contains("holds no template"));
}

#[test]
fn a_template_that_breaks_a_save_rule_is_named_with_its_reason() {
    let mut empty_command = agent_entry("Broken");
    empty_command["body"]["command"] = json!("  ");
    let zero_count =
        team_entry(&json!([{"entry": {"name_pattern": "m", "command": "c"}, "count": 0}]));
    let too_many =
        team_entry(&json!([{"entry": {"name_pattern": "m-{n}", "command": "c"}, "count": 50}]));

    let empty_command = refusal(&file(&[agent_entry("Fine"), empty_command]));
    assert!(
        empty_command.contains("template \"Broken\""),
        "{empty_command}"
    );
    assert!(empty_command.contains("the CLI command is empty"));
    assert!(
        refusal(&file(&[zero_count]))
            .contains("template \"Squad\": members[0]: the count is below 1")
    );
    assert!(
        refusal(&file(&[too_many])).contains("the team would create 51 agents; the limit is 50")
    );
}

#[test]
fn a_file_of_fifty_agents_is_accepted() {
    let fifty =
        team_entry(&json!([{"entry": {"name_pattern": "m-{n}", "command": "c"}, "count": 49}]));

    assert!(parse_file(&file(&[fifty])).is_ok());
}

#[test]
fn an_unknown_field_in_the_file_or_a_template_is_named() {
    let mut extra_file = serde_json::from_str::<Value>(&file(&[agent_entry("A")])).expect("json");
    extra_file["note"] = json!("x");
    let mut extra_template = agent_entry("A");
    extra_template["own_worktree"] = json!(true);

    assert!(
        refusal(&extra_file.to_string()).contains("file.note is not a field of a template file")
    );
    assert!(
        refusal(&file(&[extra_template]))
            .contains("templates[0].own_worktree is not a field of a template")
    );
}

#[test]
fn a_template_without_a_name_is_named_by_its_position() {
    let mut nameless = agent_entry("A");
    nameless.as_object_mut().expect("object").remove("name");

    assert!(refusal(&file(&[nameless])).contains("template \"template 1\""));
}
