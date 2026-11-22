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
/// (PB-1, HIGH-1 of Efe's PR 36 review: the field used to be dropped).
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
