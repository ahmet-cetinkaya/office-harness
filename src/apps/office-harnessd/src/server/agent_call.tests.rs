// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn a_type_within_the_limit_is_logged_whole() {
    assert_eq!(
        truncated("agent.delete", MAX_REFUSAL_TEXT_CHARS),
        "agent.delete"
    );
}

#[test]
fn a_type_over_the_limit_is_cut_and_marks_the_cut() {
    let long = "a".repeat(100);

    let logged = truncated(&long, MAX_REFUSAL_TEXT_CHARS);

    assert_eq!(logged.chars().count(), MAX_REFUSAL_TEXT_CHARS + 1);
    assert!(logged.ends_with('…'));
    assert_eq!(
        logged.trim_end_matches('…'),
        "a".repeat(MAX_REFUSAL_TEXT_CHARS)
    );
}

#[test]
fn the_limit_counts_characters_not_bytes() {
    let multibyte = "é".repeat(MAX_REFUSAL_TEXT_CHARS * 2);

    let logged = truncated(&multibyte, MAX_REFUSAL_TEXT_CHARS);

    assert_eq!(logged.chars().count(), MAX_REFUSAL_TEXT_CHARS + 1);
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

#[test]
fn an_over_long_field_key_is_cut_in_the_refusal_reason() {
    let key = "k".repeat(200);
    let mut fields = serde_json::Map::new();
    fields.insert(key.clone(), serde_json::Value::Null);

    let reason = parse_directive(&fields).expect_err("an extra field is refused");

    // The client's key keeps at most the same 64 characters as the log line,
    // so a hostile frame cannot flood the record with one key.
    let recorded = format!(
        "directive.send does not take the field {:?}",
        truncated(&key, MAX_REFUSAL_TEXT_CHARS)
    );
    assert_eq!(reason, recorded);
    assert!(reason.contains(&"k".repeat(MAX_REFUSAL_TEXT_CHARS)));
    assert!(!reason.contains(&"k".repeat(MAX_REFUSAL_TEXT_CHARS + 1)));
}

#[test]
fn a_type_over_the_limit_is_cut_in_the_not_yet_reason() {
    let kind = "agent".to_owned() + &"x".repeat(100);

    let reason = refusal_reason(&Refusal::NotYet, &kind);

    // The cut keeps MAX_REFUSAL_TEXT_CHARS characters of the type itself,
    // "agent" plus the rest of the budget in x's.
    assert!(reason.starts_with(&truncated(&kind, MAX_REFUSAL_TEXT_CHARS)));
    let kept_xs = MAX_REFUSAL_TEXT_CHARS - "agent".len();
    assert!(reason.contains(&"x".repeat(kept_xs)));
    assert!(!reason.contains(&"x".repeat(kept_xs + 1)));
}

/// An office request with only the fields its type takes, plus the caller
/// for a create's role object.
fn office_request(kind: &str, fields: &[(&str, Value)]) -> Request {
    Request {
        id: 1,
        kind: kind.to_owned(),
        fields: fields
            .iter()
            .map(|(key, value)| ((*key).to_owned(), value.clone()))
            .collect(),
    }
}

#[test]
fn an_office_request_with_an_extra_field_is_refused_not_dropped() {
    let caller = AgentId::generate();
    // A stop that smuggles `include_descendants`: dropped silently, it
    // would answer success while stopping the target alone (PR 40 LOW).
    let mut request = office_request("agent.stop", &[("agent_id", json!(AgentId::generate()))]);
    request
        .fields
        .insert("include_descendants".to_owned(), json!(true));

    let error = allowed_fields_of(&request, caller).expect_err("an extra field is refused");

    assert!(error.contains("include_descendants"), "{error}");
}

#[test]
fn a_create_with_an_extra_field_is_refused_not_dropped() {
    let caller = AgentId::generate();
    let request = office_request(
        "agent.create",
        &[
            ("name", json!("Bo")),
            ("command", json!("fake-cli")),
            ("unknown_thing", json!(1)),
        ],
    );

    let error = allowed_fields_of(&request, caller).expect_err("an extra field is refused");

    assert!(error.contains("unknown_thing"), "{error}");
}

#[test]
fn an_office_request_with_only_its_own_fields_passes_the_allowlist() {
    let caller = AgentId::generate();
    let target = AgentId::generate();
    let stop = office_request("agent.stop", &[("agent_id", json!(target.as_uuid_text()))]);
    assert!(allowed_fields_of(&stop, caller).is_ok());

    let create = office_request(
        "agent.create",
        &[
            ("name", json!("Bo")),
            ("command", json!("fake-cli")),
            ("role", json!({"kind": "member"})),
        ],
    );
    assert!(allowed_fields_of(&create, caller).is_ok());
}
