// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn keeps_the_order_of_flags_and_entries() {
    let additions = OfficeAdditions::new(
        vec!["--b".to_owned(), "--a".to_owned()],
        vec![
            EnvEntry::parse("Z", Some("1")).expect("entry"),
            EnvEntry::parse("A", Some("2")).expect("entry"),
        ],
        Some("hello".to_owned()),
    )
    .expect("valid");
    assert_eq!(additions.flags(), ["--b", "--a"]);
    assert_eq!(additions.env()[0].name(), "Z");
    assert_eq!(additions.startup_prompt(), Some("hello"));
}

#[test]
fn an_empty_flag_is_refused_by_position() {
    let flags = vec!["--ok".to_owned(), String::new()];
    assert_eq!(
        OfficeAdditions::new(flags, Vec::new(), None),
        Err(AgentError::EmptyFlag { index: 1 })
    );
}

#[test]
fn a_blank_prompt_counts_as_none() {
    let additions =
        OfficeAdditions::new(Vec::new(), Vec::new(), Some("  \n".to_owned())).expect("valid");
    assert_eq!(additions.startup_prompt(), None);
}

#[test]
fn a_nul_character_is_refused_in_a_flag_and_in_the_prompt() {
    assert!(OfficeAdditions::new(vec!["a\0".to_owned()], Vec::new(), None).is_err());
    assert!(OfficeAdditions::new(Vec::new(), Vec::new(), Some("a\0".to_owned())).is_err());
}

fn prompt_of(text: &str) -> Result<OfficeAdditions, AgentError> {
    OfficeAdditions::new(Vec::new(), Vec::new(), Some(text.to_owned()))
}

#[test]
fn trailing_line_breaks_are_trimmed_from_the_prompt() {
    let additions = prompt_of("first\nsecond\r\n\n").expect("valid");
    assert_eq!(additions.startup_prompt(), Some("first\nsecond"));
}

#[test]
fn tabs_and_inner_line_feeds_are_kept() {
    let additions = prompt_of("a\tb\nc").expect("valid");
    assert_eq!(additions.startup_prompt(), Some("a\tb\nc"));
}

#[test]
fn escape_nul_and_inner_carriage_return_are_refused_in_the_prompt() {
    for text in ["a\u{1b}[2J", "a\0b", "a\rb", "a\u{7f}", "a\u{8}b"] {
        assert!(
            matches!(
                prompt_of(text),
                Err(AgentError::ContainsControlCharacter { .. })
            ),
            "{text:?}"
        );
    }
}
