// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn a_type_within_the_limit_is_logged_whole() {
    assert_eq!(
        truncated("agent.delete", MAX_LOGGED_KIND_CHARS),
        "agent.delete"
    );
}

#[test]
fn a_type_over_the_limit_is_cut_and_marks_the_cut() {
    let long = "a".repeat(100);

    let logged = truncated(&long, MAX_LOGGED_KIND_CHARS);

    assert_eq!(logged.chars().count(), MAX_LOGGED_KIND_CHARS + 1);
    assert!(logged.ends_with('…'));
    assert_eq!(
        logged.trim_end_matches('…'),
        "a".repeat(MAX_LOGGED_KIND_CHARS)
    );
}

#[test]
fn the_limit_counts_characters_not_bytes() {
    let multibyte = "é".repeat(MAX_LOGGED_KIND_CHARS * 2);

    let logged = truncated(&multibyte, MAX_LOGGED_KIND_CHARS);

    assert_eq!(logged.chars().count(), MAX_LOGGED_KIND_CHARS + 1);
}

#[test]
fn an_actor_or_caller_field_is_smuggled_whatever_the_type() {
    let mut fields = serde_json::Map::new();
    fields.insert("actor".to_owned(), serde_json::Value::Null);

    assert_eq!(
        smuggled_field(&fields).as_deref(),
        Some("actor"),
        "an actor field is named, not taken",
    );

    let mut fields = serde_json::Map::new();
    fields.insert("caller".to_owned(), serde_json::Value::Null);

    assert_eq!(smuggled_field(&fields).as_deref(), Some("caller"));
}

#[test]
fn a_frame_without_actor_fields_is_clean() {
    let mut fields = serde_json::Map::new();
    fields.insert(
        "target_agent_id".to_owned(),
        serde_json::json!(AgentId::generate().as_uuid_text()),
    );
    fields.insert("text".to_owned(), serde_json::json!("hello"));

    assert_eq!(smuggled_field(&fields), None);
}

#[test]
fn directive_fields_are_read_and_extra_ones_rejected() {
    let target = AgentId::generate();
    let mut fields = serde_json::Map::new();
    fields.insert(
        "target_agent_id".to_owned(),
        serde_json::json!(target.as_uuid_text()),
    );
    fields.insert("text".to_owned(), serde_json::json!("ship it"));

    let parsed = parse_directive(&fields);

    assert_eq!(parsed, Ok((target, "ship it".to_owned())));

    fields.insert(
        "actor".to_owned(),
        serde_json::json!(AgentId::generate().as_uuid_text()),
    );
    let extra = parse_directive(&fields);

    assert!(extra.is_err(), "an extra field is refused");
}

#[test]
fn a_bad_target_or_text_shape_is_a_parse_failure() {
    let mut fields = serde_json::Map::new();
    fields.insert(
        "target_agent_id".to_owned(),
        serde_json::json!("not-a-uuid"),
    );
    fields.insert("text".to_owned(), serde_json::json!("hello"));

    assert!(parse_directive(&fields).is_err());

    let mut fields = serde_json::Map::new();
    fields.insert(
        "target_agent_id".to_owned(),
        serde_json::json!(AgentId::generate().as_uuid_text()),
    );
    fields.insert("text".to_owned(), serde_json::json!(42));

    assert!(parse_directive(&fields).is_err());
}
