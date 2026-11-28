// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::json;

use super::*;

fn map(value: Value) -> Map<String, Value> {
    let Value::Object(map) = value else {
        panic!("an object")
    };
    map
}

#[test]
fn a_missing_required_field_is_named() {
    let body = map(json!({}));

    assert_eq!(
        Fields(&body).required_text("name"),
        Err("name is required".to_owned())
    );
}

#[test]
fn a_field_of_the_wrong_type_is_named() {
    let body = map(json!({"name": 5, "folder": ["x"]}));

    assert_eq!(
        Fields(&body).required_text("name"),
        Err("name must be text".to_owned())
    );
    assert_eq!(
        Fields(&body).optional_text("folder"),
        Err("folder must be text".to_owned())
    );
}

#[test]
fn an_optional_field_may_be_absent_or_null() {
    let body = map(json!({"project_id": null}));

    assert_eq!(Fields(&body).optional_text("project_id"), Ok(None));
    assert_eq!(Fields(&body).optional_text("working_folder"), Ok(None));
}

/// The `role` object of `agent.create` reads into its three fields, and a
/// `role` the request never carries stays absent, the exact 0001 behavior
/// (PB-1: the field used to be dropped).
#[test]
fn the_role_object_reads_whole_or_is_absent() {
    use super::super::agent_requests::CreateAgentRequest;
    use serde_json::json;

    let body = json!({
        "name": "Ana",
        "command": "claude",
        "role": {"kind": "member", "parent_agent_id": "abc", "team_id": null},
    });
    let body = body.as_object().expect("object");
    let read = CreateAgentRequest::read(body).expect("reads");
    let role = read.role.expect("the role is read");
    assert_eq!(role.kind, "member");
    assert_eq!(role.parent_agent_id.as_deref(), Some("abc"));
    assert_eq!(role.team_id, None);

    let plain = json!({"name": "Ana", "command": "claude"});
    let read = CreateAgentRequest::read(plain.as_object().expect("object")).expect("reads");
    assert!(read.role.is_none(), "no role object is no role");
}

#[test]
fn a_role_that_is_not_an_object_is_named() {
    use super::super::agent_requests::CreateAgentRequest;
    use serde_json::json;

    let body = json!({"name": "Ana", "command": "claude", "role": "ceo"});
    let error = CreateAgentRequest::read(body.as_object().expect("object")).expect_err("refused");
    assert_eq!(error, "role must be an object");

    let body = json!({"name": "Ana", "command": "claude", "role": {}});
    let error = CreateAgentRequest::read(body.as_object().expect("object")).expect_err("refused");
    assert_eq!(error, "role.kind is required");
}

#[test]
fn a_whole_number_field_is_read_and_a_bad_one_is_named() {
    let body = map(
        json!({"skip": 7, "negative": -1, "fraction": 1.5, "text": "7", "huge": 4_294_967_296_u64, "nothing": null}),
    );
    let fields = Fields(&body);

    assert_eq!(fields.optional_u32("skip"), Ok(Some(7)));
    assert_eq!(fields.optional_u32("nothing"), Ok(None));
    assert_eq!(fields.optional_u32("absent"), Ok(None));
    for name in ["negative", "fraction", "text", "huge"] {
        assert_eq!(
            fields.optional_u32(name),
            Err(format!("{name} must be a whole number of at least 0"))
        );
    }
}

fn create_body(extra: Value) -> Map<String, Value> {
    let mut body = map(json!({"name": "Ana", "command": "claude"}));
    body.extend(map(extra));
    body
}

#[test]
fn a_create_reads_the_temp_objective_only_with_temp_on() {
    use super::super::agent_requests::CreateAgentRequest;

    let read = CreateAgentRequest::read(&create_body(json!({"temp": true, "objective": "go"})))
        .expect("reads");
    assert_eq!(read.temp_objective.as_deref(), Some("go"));

    for plain in [json!({}), json!({"temp": false}), json!({"temp": null})] {
        let read = CreateAgentRequest::read(&create_body(plain)).expect("reads");
        assert_eq!(read.temp_objective, None);
    }
}

#[test]
fn a_temp_without_an_objective_and_an_objective_without_a_temp_are_invalid() {
    use super::super::agent_requests::CreateAgentRequest;

    let cases = [
        (json!({"temp": true}), "a temp needs an objective"),
        (
            json!({"objective": "go"}),
            "objective is only taken with temp",
        ),
        (
            json!({"temp": false, "objective": "go"}),
            "objective is only taken with temp",
        ),
        (
            json!({"temp": "yes", "objective": "go"}),
            "temp must be true or false",
        ),
    ];
    for (extra, message) in cases {
        let error = CreateAgentRequest::read(&create_body(extra)).expect_err(message);
        assert_eq!(error, message);
    }
}

#[test]
fn the_temp_end_and_dismiss_bodies_take_one_field_and_refuse_others() {
    use super::super::agent_requests::{TempDismissRequest, TempEndRequest};

    let end = TempEndRequest::read(&map(json!({"agent_id": "a1"}))).expect("reads");
    assert_eq!(end.agent_id, "a1");
    let dismiss = TempDismissRequest::read(&map(json!({"entry_id": "e1"}))).expect("reads");
    assert_eq!(dismiss.entry_id, "e1");

    assert_eq!(
        TempEndRequest::read(&map(json!({}))).expect_err("missing"),
        "agent_id is required"
    );
    assert_eq!(
        TempEndRequest::read(&map(json!({"agent_id": "a1", "force": true}))).expect_err("extra"),
        "force is not a field of temp.end"
    );
    assert_eq!(
        TempDismissRequest::read(&map(json!({"entry_id": "e1", "all": true}))).expect_err("extra"),
        "all is not a field of temp.dismiss"
    );
}
