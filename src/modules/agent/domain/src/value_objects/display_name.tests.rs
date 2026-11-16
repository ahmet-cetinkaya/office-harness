// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn trims_the_name() {
    let name = DisplayName::parse("name", "  Ana  ").expect("valid");
    assert_eq!(name.as_str(), "Ana");
}

#[test]
fn rejects_a_blank_name_and_names_the_field() {
    let error = DisplayName::parse("agent name", "   ").expect_err("blank");
    assert_eq!(error.to_string(), "agent name must not be empty");
}

#[test]
fn accepts_64_characters_and_rejects_65() {
    assert!(DisplayName::parse("name", &"ü".repeat(64)).is_ok());
    assert_eq!(
        DisplayName::parse("name", &"ü".repeat(65)),
        Err(AgentError::TooLong {
            field: "name",
            max: 64
        })
    );
}

#[test]
fn uniqueness_ignores_case_beyond_ascii() {
    let first = DisplayName::parse("name", "ÜNAL").expect("valid");
    let second = DisplayName::parse("name", "ünal").expect("valid");
    assert_eq!(first.uniqueness_key(), second.uniqueness_key());
}
