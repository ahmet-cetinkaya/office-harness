// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn parse(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or(Value::Null)
}

#[test]
fn a_request_keeps_its_extra_fields() {
    let request = Request::decode(r#"{"id":7,"type":"agent.delete","agent_id":"a1"}"#)
        .unwrap_or_else(|| Request {
            id: 0,
            kind: String::new(),
            fields: Map::new(),
        });

    assert_eq!(request.id, 7);
    assert_eq!(request.kind, "agent.delete");
    assert_eq!(request.fields.get("agent_id"), Some(&json!("a1")));
}

#[test]
fn frames_that_are_not_requests_are_none() {
    for frame in [
        "",
        "[]",
        r#"{"type":"x"}"#,
        r#"{"id":-1,"type":"x"}"#,
        r#"{"id":"1","type":"x"}"#,
    ] {
        assert_eq!(Request::decode(frame), None, "{frame}");
    }
}

#[test]
fn a_success_and_a_failure_follow_the_contract_shape() {
    assert_eq!(
        parse(&Response::success(3, json!({"n": 1})).encode()),
        json!({"id": 3, "ok": true, "result": {"n": 1}})
    );
    assert_eq!(
        parse(&Response::failure(4, ErrorCode::NotFound, "no such agent").encode()),
        json!({"id": 4, "ok": false, "error": {"code": "not_found", "message": "no such agent"}})
    );
}

#[test]
fn an_event_has_a_type_and_no_id() {
    let mut fields = Map::new();
    fields.insert("message".to_owned(), json!("disk full"));

    assert_eq!(
        parse(&Event::new("storage.error", fields).encode()),
        json!({"type": "storage.error", "message": "disk full"})
    );
}
