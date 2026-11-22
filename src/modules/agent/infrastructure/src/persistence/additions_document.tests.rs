// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn additions_survive_their_stored_text() {
    let additions = OfficeAdditions::new(
        vec!["--a".to_owned()],
        vec![EnvEntry::parse("K", Some("v")).expect("entry")],
        Some("prompt\nline".to_owned()),
    )
    .expect("additions");

    let text = AdditionsDocument::text_of(&additions).expect("text");

    assert_eq!(AdditionsDocument::parse(&text).expect("parse"), additions);
}

#[test]
fn a_damaged_row_is_refused_by_the_domain_rules() {
    let text = r#"{"flags":[],"env":[["1BAD","x"]],"startup_prompt":null}"#;

    assert!(matches!(
        AdditionsDocument::parse(text),
        Err(PersistenceError::Invalid(_))
    ));
    assert!(matches!(
        AdditionsDocument::parse("not json"),
        Err(PersistenceError::InvalidJson(_))
    ));
}

#[test]
fn no_value_and_an_empty_value_stay_different_in_the_stored_text() {
    let additions = OfficeAdditions::new(
        Vec::new(),
        vec![
            EnvEntry::parse("NONE", None).expect("entry"),
            EnvEntry::parse("EMPTY", Some("")).expect("entry"),
        ],
        None,
    )
    .expect("additions");

    let text = AdditionsDocument::text_of(&additions).expect("text");

    assert!(text.contains(r#"["NONE",null]"#));
    assert!(text.contains(r#"["EMPTY",""]"#));
    assert_eq!(AdditionsDocument::parse(&text).expect("parse"), additions);
}

#[test]
fn an_older_row_with_text_values_loads_unchanged() {
    let text = r#"{"flags":[],"env":[["KEY","value"]],"startup_prompt":null}"#;

    let additions = AdditionsDocument::parse(text).expect("parse");

    assert_eq!(additions.env()[0].value(), Some("value"));
}
