// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn the_caller_cannot_override_the_type_or_the_id() {
    let mut fields = Map::new();
    fields.insert("type".to_owned(), Value::String("agent.delete".to_owned()));
    fields.insert("id".to_owned(), Value::from(99));
    fields.insert("agent_id".to_owned(), Value::String("a1".to_owned()));
    let frame: Value = serde_json::from_str(&request_frame("agent.whoami", fields)).expect("json");
    assert_eq!(frame["type"], "agent.whoami");
    assert_eq!(frame["id"], REQUEST_ID);
    assert_eq!(frame["agent_id"], "a1");
}

#[test]
fn fields_must_be_a_json_object() {
    assert!(read_fields(None).expect("no fields").is_empty());
    let object = OsString::from(r#"{"name":"x"}"#);
    assert_eq!(read_fields(Some(&object)).expect("object").len(), 1);
    for bad in ["[1]", "\"x\"", "not json", "3"] {
        let text = OsString::from(bad);
        assert!(
            matches!(read_fields(Some(&text)), Err(CallError::BadFields)),
            "{bad}"
        );
    }
}

#[test]
fn a_credential_is_an_agent_id_and_a_secret() {
    assert_eq!(split_credential("a1:ff").expect("split"), ("a1", "ff"));
    assert!(matches!(
        split_credential("nosecret"),
        Err(CallError::BadCredential)
    ));
}
