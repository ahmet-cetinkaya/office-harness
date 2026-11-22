// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;
use serde_json::{Value, json};
use wire::{ErrorCode, Request};

use super::{ResumeGateway, agent_ids_of, refuse_unknown_fields};

fn request(kind: &str, fields: &Value) -> Request {
    Request {
        id: 1,
        kind: kind.to_owned(),
        fields: fields.as_object().cloned().unwrap_or_default(),
    }
}

fn refusal_of(fields: &Value) -> String {
    match agent_ids_of(&request("resume.start", fields)) {
        Err((ErrorCode::Invalid, message)) => message,
        other => panic!("expected an invalid refusal, got {other:?}"),
    }
}

#[test]
fn the_gateway_takes_only_the_two_resume_requests() {
    assert!(ResumeGateway::accepts("resume.plan"));
    assert!(ResumeGateway::accepts("resume.start"));
    assert!(!ResumeGateway::accepts("resume.other"));
    assert!(!ResumeGateway::accepts("agent.resume"));
}

#[test]
fn agent_ids_are_read_in_request_order() {
    let [first, second] = std::array::from_fn(|_| AgentId::generate());
    let fields = json!({"agent_ids": [first.to_string(), second.to_string()]});

    let ids = agent_ids_of(&request("resume.start", &fields)).expect("ids");

    assert_eq!(ids, [first, second]);
}

#[test]
fn an_empty_list_a_duplicate_and_a_bad_id_are_refused() {
    let agent_id = AgentId::generate().to_string();

    assert!(refusal_of(&json!({"agent_ids": []})).contains("1 to 1000"));
    assert!(refusal_of(&json!({"agent_ids": [agent_id, agent_id]})).contains("twice"));
    assert!(refusal_of(&json!({"agent_ids": ["not-an-id"]})).contains("not an agent id"));
    assert!(refusal_of(&json!({"agent_ids": "x"})).contains("a list"));
}

#[test]
fn more_than_a_thousand_ids_are_refused() {
    let ids: Vec<String> = (0..1001).map(|_| AgentId::generate().to_string()).collect();

    assert!(refusal_of(&json!({ "agent_ids": ids })).contains("1 to 1000"));
}

#[test]
fn a_field_the_request_does_not_take_is_refused() {
    let plan = refuse_unknown_fields(&request("resume.plan", &json!({"agent_id": "x"})));
    let start = refuse_unknown_fields(&request("resume.start", &json!({"agent_idz": []})));

    assert_eq!(
        plan,
        Err((
            ErrorCode::Invalid,
            "agent_id is not a field of resume.plan".to_owned()
        ))
    );
    assert!(matches!(start, Err((ErrorCode::Invalid, message)) if message.contains("agent_idz")));
}
