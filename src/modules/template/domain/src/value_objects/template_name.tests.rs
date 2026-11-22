// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn a_name_is_trimmed_and_its_key_ignores_case() {
    let name = TemplateName::parse("  Reviewer ").expect("name");

    assert_eq!(name.as_str(), "Reviewer");
    assert_eq!(name.key(), "reviewer");
}

#[test]
fn a_blank_long_or_control_name_is_refused() {
    assert_eq!(TemplateName::parse(" \t"), Err(TemplateError::NameEmpty));
    assert_eq!(
        TemplateName::parse(&"x".repeat(MAX_NAME_CHARS + 1)),
        Err(TemplateError::NameTooLong {
            max: MAX_NAME_CHARS
        })
    );
    assert_eq!(
        TemplateName::parse("a\u{1b}b"),
        Err(TemplateError::NameHasControlCharacter)
    );
    assert!(TemplateName::parse(&"x".repeat(MAX_NAME_CHARS)).is_ok());
}

#[test]
fn a_numbered_name_keeps_within_the_length_rule() {
    let name = TemplateName::parse("Reviewer").expect("name");
    assert_eq!(name.numbered(2).as_str(), "Reviewer (2)");

    let long = TemplateName::parse(&"x".repeat(MAX_NAME_CHARS)).expect("name");
    let numbered = long.numbered(12);
    assert_eq!(numbered.as_str().chars().count(), MAX_NAME_CHARS);
    assert!(numbered.as_str().ends_with(" (12)"));
}
