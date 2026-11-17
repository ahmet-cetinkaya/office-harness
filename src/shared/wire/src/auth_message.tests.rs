// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn each_message_round_trips_with_the_contract_field_names() {
    let messages = [
        (
            AuthMessage::Challenge {
                nonce: "ab".to_owned(),
            },
            r#"{"nonce":"ab","type":"auth.challenge"}"#,
        ),
        (
            AuthMessage::Auth {
                proof: "cd".to_owned(),
                agent_id: None,
            },
            r#"{"proof":"cd","type":"auth"}"#,
        ),
        (
            AuthMessage::Auth {
                proof: "cd".to_owned(),
                agent_id: Some("a1".to_owned()),
            },
            r#"{"agent_id":"a1","proof":"cd","type":"auth"}"#,
        ),
        (
            AuthMessage::AuthOk {
                proof: "ef".to_owned(),
            },
            r#"{"proof":"ef","type":"auth.ok"}"#,
        ),
    ];

    for (message, text) in messages {
        assert_eq!(message.encode(), text);
        assert_eq!(AuthMessage::decode(text), Some(message));
    }
}

#[test]
fn other_frames_are_not_auth_messages() {
    for frame in [
        "",
        "{}",
        "[]",
        "not json",
        r#"{"id":1,"type":"agent.list"}"#,
        r#"{"type":"auth"}"#,
    ] {
        assert_eq!(AuthMessage::decode(frame), None, "{frame}");
    }
}
