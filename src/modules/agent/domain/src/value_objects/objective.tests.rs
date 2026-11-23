// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn the_text_rules_of_a_directive_hold() {
    let cases: [(&str, String, Result<(), AgentError>); 8] = [
        ("one character", "x".into(), Ok(())),
        ("the limit", "x".repeat(OBJECTIVE_MAX_CHARS), Ok(())),
        (
            "empty",
            String::new(),
            Err(AgentError::Empty { field: FIELD }),
        ),
        (
            "blank",
            " \t ".into(),
            Err(AgentError::Empty { field: FIELD }),
        ),
        (
            "past the limit",
            "x".repeat(OBJECTIVE_MAX_CHARS + 1),
            Err(AgentError::TooLong {
                field: FIELD,
                max: OBJECTIVE_MAX_CHARS,
            }),
        ),
        (
            "escape",
            "red \u{1b}[31m".into(),
            Err(AgentError::ContainsControlCharacter {
                field: FIELD.into(),
            }),
        ),
        (
            "delete",
            "a\u{7f}".into(),
            Err(AgentError::ContainsControlCharacter {
                field: FIELD.into(),
            }),
        ),
        (
            "C1 control",
            "a\u{9b}".into(),
            Err(AgentError::ContainsControlCharacter {
                field: FIELD.into(),
            }),
        ),
    ];
    for (name, text, expected) in cases {
        assert_eq!(Objective::new(&text).map(|_| ()), expected, "{name}");
    }
}

#[test]
fn the_limit_counts_characters_not_bytes() {
    let wide = "\u{1F600}".repeat(OBJECTIVE_MAX_CHARS);
    assert!(Objective::new(&wide).is_ok());
}

#[test]
fn the_text_is_kept_as_entered() {
    let objective = Objective::new("  fix the build ").expect("valid");
    assert_eq!(objective.as_str(), "  fix the build ");
}
