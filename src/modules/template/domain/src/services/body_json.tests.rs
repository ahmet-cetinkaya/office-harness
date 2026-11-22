// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::json;

use super::*;

fn agent_json() -> Value {
    json!({
        "body_version": 1,
        "name_pattern": "reviewer-{n}",
        "command": "claude",
        "working_folder": "/work",
        "additions": {
            "flags": ["--verbose"],
            "env": [{"name": "A", "value": null}, {"name": "B", "value": ""}],
            "startup_prompt": "go"
        },
        "role": "member"
    })
}

fn entry_json() -> Value {
    json!({"name_pattern": "{team}-member-{n}", "command": "claude"})
}

#[test]
fn an_agent_body_round_trips_and_keeps_no_value_apart_from_an_empty_value() {
    let body = parse_body(TemplateKind::Agent, &agent_json()).expect("body");

    let TemplateBody::Agent(agent) = &body else {
        panic!("not an agent body");
    };
    assert_eq!(agent.additions.env[0].value, None);
    assert_eq!(agent.additions.env[1].value.as_deref(), Some(""));
    assert_eq!(agent.role, Some(TemplateRole::Member));
    assert_eq!(
        parse_body(TemplateKind::Agent, &body_to_json(&body)).expect("again"),
        body
    );
}

#[test]
fn a_team_body_round_trips() {
    let value = json!({
        "body_version": 1,
        "lead": entry_json(),
        "members": [{"entry": entry_json(), "count": 6}]
    });

    let body = parse_body(TemplateKind::Team, &value).expect("body");

    assert_eq!(
        parse_body(TemplateKind::Team, &body_to_json(&body)).expect("again"),
        body
    );
}

#[test]
fn an_unknown_field_is_refused_and_named_at_every_depth() {
    let mut top = agent_json();
    top["own_worktree"] = json!(true);
    let mut nested = agent_json();
    nested["additions"]["colour"] = json!("red");
    let mut env = agent_json();
    env["additions"]["env"][0]["secret"] = json!(true);
    let mut member = json!({
        "body_version": 1,
        "lead": entry_json(),
        "members": [{"entry": entry_json(), "count": 1, "extra": 1}]
    });
    member["lead"]["speed"] = json!(1);

    for (kind, body, path) in [
        (TemplateKind::Agent, top, "body.own_worktree"),
        (TemplateKind::Agent, nested, "body.additions.colour"),
        (TemplateKind::Agent, env, "body.additions.env[0].secret"),
        (TemplateKind::Team, member.clone(), "body.lead.speed"),
    ] {
        let error = parse_body(kind, &body).expect_err(path);
        assert!(error.to_string().contains(path), "{error}");
    }
    member["lead"] = entry_json();
    let error = parse_body(TemplateKind::Team, &member).expect_err("member");
    assert!(
        error.to_string().contains("body.members[0].extra"),
        "{error}"
    );
}

#[test]
fn a_wrong_shape_or_a_missing_field_names_its_path() {
    let cases = [
        (
            json!({"body_version": 1, "command": "x"}),
            "body.name_pattern is required",
        ),
        (
            json!({"body_version": 1, "name_pattern": "x", "command": 5}),
            "body.command must be text",
        ),
        (
            json!({"body_version": 1, "name_pattern": "x", "command": "c", "role": "boss"}),
            "the role \"boss\"",
        ),
        (
            json!({"body_version": 2, "name_pattern": "x", "command": "c"}),
            "the body version 2 is not supported",
        ),
    ];
    for (body, message) in cases {
        let error = parse_body(TemplateKind::Agent, &body).expect_err(message);
        assert!(error.to_string().contains(message), "{error}");
    }
}

#[test]
fn a_member_count_that_is_not_a_whole_number_is_refused() {
    let value = json!({
        "body_version": 1,
        "lead": entry_json(),
        "members": [{"entry": entry_json(), "count": -1}]
    });

    let error = parse_body(TemplateKind::Team, &value).expect_err("count");

    assert!(
        error
            .to_string()
            .contains("body.members[0].count must be a whole number")
    );
}
