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

fn arguments(texts: &[&str]) -> Vec<OsString> {
    texts.iter().map(OsString::from).collect()
}

#[test]
fn the_value_flag_is_taken_out_wherever_it_stands_and_only_for_a_secret_read() {
    let (value_only, rest) = split_value_only(&arguments(&[
        "secret.get",
        "--value-only",
        r#"{"name":"A_KEY"}"#,
    ]))
    .expect("split");
    assert!(value_only);
    assert_eq!(rest, arguments(&["secret.get", r#"{"name":"A_KEY"}"#]));

    let (value_only, rest) = split_value_only(&arguments(&["agent.whoami"])).expect("split");
    assert!(!value_only);
    assert_eq!(rest, arguments(&["agent.whoami"]));

    assert!(matches!(
        split_value_only(&arguments(&["agent.whoami", "--value-only"])),
        Err(CallError::Usage)
    ));
}

#[test]
fn an_accepted_read_gives_the_value_alone_with_no_line_break() {
    let answer =
        r#"{"id":1,"ok":true,"result":{"name":"A_KEY","value":"marker-value-9","version":2}}"#;

    assert_eq!(
        render_answer(answer, true),
        Rendered {
            accepted: true,
            stdout: Some("marker-value-9".to_owned()),
            stderr: None,
        }
    );
}

#[test]
fn a_refused_read_gives_nothing_for_standard_output() {
    let answer = r#"{"id":1,"ok":false,"error":{"code":"not_found","message":"no secret named A_KEY is available to this agent"}}"#;

    let rendered = render_answer(answer, true);

    assert!(!rendered.accepted);
    assert_eq!(rendered.stdout, None);
    assert!(
        rendered
            .stderr
            .expect("error")
            .contains("no secret named A_KEY")
    );
}

#[test]
fn without_the_flag_the_answer_is_printed_as_it_came() {
    let answer = r#"{"id":1,"ok":true,"result":{}}"#;

    let rendered = render_answer(answer, false);

    assert_eq!(rendered.stdout, Some(format!("{answer}\n")));
    assert!(rendered.accepted);
}
