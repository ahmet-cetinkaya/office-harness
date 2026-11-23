// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn a_name_in_the_form_is_accepted() {
    for text in ["A", "OPENAI_API_KEY", "K9", &"A".repeat(MAX_NAME_CHARS)] {
        assert_eq!(SecretName::parse(text).expect("name").as_str(), text);
    }
}

#[test]
fn an_empty_long_lower_case_or_spaced_name_is_refused() {
    assert_eq!(SecretName::parse(""), Err(SecretError::NameEmpty));
    assert_eq!(
        SecretName::parse(&"A".repeat(MAX_NAME_CHARS + 1)),
        Err(SecretError::NameTooLong {
            max: MAX_NAME_CHARS
        })
    );
    for text in [
        "key",
        "9KEY",
        "_KEY",
        "MY KEY",
        "MY-KEY",
        "KEY\n",
        "ANAHTAR\u{130}",
    ] {
        assert_eq!(
            SecretName::parse(text),
            Err(SecretError::NameBadForm),
            "{text:?}"
        );
    }
}
