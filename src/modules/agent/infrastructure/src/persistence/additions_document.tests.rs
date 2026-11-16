// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn additions_survive_their_stored_text() {
    let additions = OfficeAdditions::new(
        vec!["--a".to_owned()],
        vec![EnvEntry::parse("K", "v").expect("entry")],
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
