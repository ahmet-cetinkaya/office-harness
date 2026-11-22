// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::json;

use super::*;

#[test]
fn a_full_body_is_read_in_order() {
    let body = json!({
        "flags": ["--a", "--b"],
        "env": [{"name": "Z", "value": "1"}, {"name": "A", "value": "2"}],
        "startup_prompt": "go"
    });

    let input = read_additions(&body).expect("read");

    assert_eq!(input.flags, ["--a", "--b"]);
    assert_eq!(
        input.env,
        [
            ("Z".to_owned(), Some("1".to_owned())),
            ("A".to_owned(), Some("2".to_owned()))
        ]
    );
    assert_eq!(input.startup_prompt.as_deref(), Some("go"));
}

#[test]
fn a_null_value_is_no_value_and_an_empty_text_is_a_value() {
    let body = json!({"env": [{"name": "A", "value": null}, {"name": "B", "value": ""}]});

    let input = read_additions(&body).expect("read");

    assert_eq!(
        input.env,
        [
            ("A".to_owned(), None),
            ("B".to_owned(), Some(String::new()))
        ]
    );
}

#[test]
fn missing_parts_are_empty() {
    assert_eq!(read_additions(&json!({})), Ok(AdditionsInput::default()));
    assert_eq!(
        read_additions(&json!({"flags": null, "env": null, "startup_prompt": null})),
        Ok(AdditionsInput::default())
    );
}

#[test]
fn each_failure_names_the_part_at_fault() {
    let cases = [
        (json!("x"), "additions must be an object"),
        (json!({"flags": "x"}), "additions.flags must be a list"),
        (
            json!({"flags": ["a", 5]}),
            "additions.flags[1] must be text",
        ),
        (json!({"env": {}}), "additions.env must be a list"),
        (json!({"env": [5]}), "additions.env[0] must be an object"),
        (
            json!({"env": [{"value": "v"}]}),
            "additions.env[0].name is required",
        ),
        (
            json!({"env": [{"name": "A", "value": 1}]}),
            "additions.env[0].value must be text",
        ),
        (
            json!({"startup_prompt": 3}),
            "additions.startup_prompt must be text",
        ),
    ];
    for (body, message) in cases {
        assert_eq!(read_additions(&body), Err(message.to_owned()), "{body}");
    }
}
