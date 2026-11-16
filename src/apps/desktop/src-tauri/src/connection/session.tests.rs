// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::json;

use super::*;

fn fields(value: &Value) -> Map<String, Value> {
    value.as_object().cloned().expect("an object")
}

#[test]
fn the_request_frame_carries_the_fields_and_cannot_be_overridden() {
    let sent = fields(&json!({"agent_id": "a", "id": 99, "type": "x"}));

    let frame = encode_request(7, "agent.start", sent);

    let decoded: Value = serde_json::from_str(&frame).expect("json");
    assert_eq!(
        decoded,
        json!({"id": 7, "type": "agent.start", "agent_id": "a"})
    );
}

#[test]
fn a_success_is_a_response_with_its_result() {
    let inbound = classify_frame(r#"{"id":3,"ok":true,"result":[1]}"#);

    assert_eq!(
        inbound,
        Inbound::Response {
            id: 3,
            outcome: Ok(json!([1]))
        }
    );
}

#[test]
fn a_success_without_a_result_is_null() {
    let inbound = classify_frame(r#"{"id":3,"ok":true}"#);

    assert_eq!(
        inbound,
        Inbound::Response {
            id: 3,
            outcome: Ok(Value::Null)
        }
    );
}

#[test]
fn a_failure_keeps_the_code_and_the_message() {
    let inbound =
        classify_frame(r#"{"id":4,"ok":false,"error":{"code":"conflict","message":"name taken"}}"#);

    assert_eq!(
        inbound,
        Inbound::Response {
            id: 4,
            outcome: Err(ApiError::new("conflict", "name taken"))
        }
    );
}

#[test]
fn a_frame_without_an_id_is_an_event() {
    let inbound = classify_frame(r#"{"type":"agent.status","agent_id":"a"}"#);

    assert_eq!(
        inbound,
        Inbound::Event(json!({"type": "agent.status", "agent_id": "a"}))
    );
}

#[test]
fn text_that_is_neither_is_ignored() {
    assert_eq!(classify_frame("not json"), Inbound::Ignored);
    assert_eq!(classify_frame("[1]"), Inbound::Ignored);
    assert_eq!(classify_frame(r#"{"other":1}"#), Inbound::Ignored);
    assert_eq!(
        classify_frame(r#"{"id":99999999999,"ok":true}"#),
        Inbound::Ignored
    );
}
