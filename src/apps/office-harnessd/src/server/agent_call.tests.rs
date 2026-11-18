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

use hierarchy_domain::TreeRow;

/// The tree rows a test reads: `caller` placed as `caller_role`, `other`
/// as a lead under the caller when `other` is given, so a descendant-lead
/// parent has rows to be checked against.
fn rows_of(caller: AgentId, caller_role: Role, other: Option<(AgentId, Role)>) -> Vec<TreeRow> {
    let mut rows = vec![TreeRow {
        agent_id: caller,
        name: "Caller".to_owned(),
        role: Some(caller_role),
        parent_agent_id: None,
        team_id: None,
    }];
    if let Some((agent_id, role)) = other {
        rows.push(TreeRow {
            agent_id,
            name: "Other".to_owned(),
            role: Some(role),
            parent_agent_id: Some(caller),
            team_id: None,
        });
    }
    rows
}

/// A create frame whose fields are `name`, `command`, and the given role
/// value; `Value::Null` omits the role.
fn create_with_role(role: &Value) -> serde_json::Map<String, Value> {
    let mut fields: serde_json::Map<String, Value> = serde_json::from_value(json!({
        "name": "Bo",
        "command": "cli",
        "role": role,
    }))
    .expect("fields");
    if role.is_null() {
        fields.remove("role");
    }
    fields
}

fn refused_of(
    fields: &serde_json::Map<String, Value>,
    caller: AgentId,
    rows: &[TreeRow],
) -> String {
    let Err((ErrorCode::Refused, message)) =
        allowed_fields_of("agent.create", fields, caller, rows)
    else {
        panic!("expected a refused outcome");
    };
    message
}

#[test]
fn a_ceo_callers_create_keeps_the_requested_lead_role() {
    let caller = AgentId::generate();
    let fields = create_with_role(&json!({"kind": "lead"}));

    let fields = allowed_fields_of(
        "agent.create",
        &fields,
        caller,
        &rows_of(caller, Role::Ceo, None),
    )
    .expect("a ceo may raise a lead");

    assert_eq!(
        fields["role"],
        json!({"kind": "lead"}),
        "the lead takes its parent from the tree, so none travels"
    );
}

#[test]
fn a_lead_callers_create_keeps_the_requested_member_parent() {
    let caller = AgentId::generate();
    let descendant_lead = AgentId::generate();
    let rows = rows_of(caller, Role::Ceo, Some((descendant_lead, Role::Lead)));
    let fields = create_with_role(&json!({
        "kind": "member",
        "parent_agent_id": descendant_lead.as_uuid_text(),
    }));

    let fields = allowed_fields_of("agent.create", &fields, caller, &rows)
        .expect("a ceo may raise a member under its own descendant lead");

    assert_eq!(
        fields["role"],
        json!({"kind": "member", "parent_agent_id": descendant_lead.as_uuid_text()}),
        "the member lands under the descendant lead the caller named"
    );
}

#[test]
fn a_named_parent_that_is_not_a_lead_is_refused() {
    let caller = AgentId::generate();
    let child_member = AgentId::generate();
    let rows = rows_of(caller, Role::Lead, Some((child_member, Role::Member)));
    let fields = create_with_role(&json!({
        "kind": "member",
        "parent_agent_id": child_member.as_uuid_text(),
    }));

    let message = refused_of(&fields, caller, &rows);

    assert!(message.contains("not a lead"), "{message}");
}

#[test]
fn a_ceo_caller_may_raise_a_member_under_its_descendant_lead() {
    let caller = AgentId::generate();
    let team_lead = AgentId::generate();
    let rows = rows_of(caller, Role::Ceo, Some((team_lead, Role::Lead)));
    let fields = create_with_role(&json!({
        "kind": "member",
        "parent_agent_id": team_lead.as_uuid_text(),
    }));

    let fields = allowed_fields_of("agent.create", &fields, caller, &rows)
        .expect("a ceo may fill its team's seats through the team's lead");

    assert_eq!(
        fields["role"],
        json!({"kind": "member", "parent_agent_id": team_lead.as_uuid_text()}),
        "the member lands under the lead the ceo named"
    );
}

#[test]
fn a_create_without_a_role_defaults_to_a_member_under_the_caller() {
    let caller = AgentId::generate();
    let fields = create_with_role(&Value::Null);

    let fields = allowed_fields_of(
        "agent.create",
        &fields,
        caller,
        &rows_of(caller, Role::Lead, None),
    )
    .expect("allowed");

    assert_eq!(
        fields["role"],
        json!({"kind": "member", "parent_agent_id": caller.as_uuid_text()}),
        "the plain create keeps its member-under-the-caller meaning"
    );
}

#[test]
fn a_member_caller_asking_for_any_child_role_is_refused() {
    let caller = AgentId::generate();
    for kind in ["lead", "member"] {
        let fields = create_with_role(&json!({"kind": kind}));

        let message = refused_of(&fields, caller, &rows_of(caller, Role::Member, None));

        assert!(
            message.contains("a member may not create agents"),
            "{kind}: {message}"
        );
    }
}

#[test]
fn a_lead_caller_asking_for_a_lead_is_refused_with_its_allowed_roles() {
    let caller = AgentId::generate();
    let fields = create_with_role(&json!({"kind": "lead"}));

    let message = refused_of(&fields, caller, &rows_of(caller, Role::Lead, None));

    assert!(message.contains("member"), "{message}");
}

#[test]
fn an_unplaced_caller_asking_for_a_role_is_refused() {
    let caller = AgentId::generate();
    // No row for the caller: the agent has no place yet.
    let fields = create_with_role(&json!({"kind": "member"}));

    let message = refused_of(&fields, caller, &[]);

    assert!(message.contains("place"), "{message}");
}

#[test]
fn no_caller_role_makes_a_ceo_child() {
    for caller_role in [None, Some(Role::Member), Some(Role::Lead), Some(Role::Ceo)] {
        let caller = AgentId::generate();
        let fields = create_with_role(&json!({"kind": "ceo"}));
        let rows = caller_role.map_or_default(|role| rows_of(caller, role, None));

        let message = refused_of(&fields, caller, &rows);

        assert!(message.contains("ceo"), "{caller_role:?}: {message}");
    }
}

#[test]
fn a_lead_role_with_a_parent_is_invalid() {
    let caller = AgentId::generate();
    let fields = create_with_role(&json!({
        "kind": "lead",
        "parent_agent_id": AgentId::generate().as_uuid_text(),
    }));

    let Err((ErrorCode::Invalid, message)) = allowed_fields_of(
        "agent.create",
        &fields,
        caller,
        &rows_of(caller, Role::Ceo, None),
    ) else {
        panic!("expected an invalid outcome");
    };

    assert!(message.contains("role.parent_agent_id"), "{message}");
}

#[test]
fn a_role_of_the_wrong_shape_or_name_is_invalid() {
    let caller = AgentId::generate();
    for role in [
        json!("lead"),
        json!({}),
        json!({"kind": "emperor"}),
        json!({"kind": "member", "parent_agent_id": "not-a-uuid"}),
    ] {
        let fields = create_with_role(&role);

        let Err((ErrorCode::Invalid, message)) = allowed_fields_of(
            "agent.create",
            &fields,
            caller,
            &rows_of(caller, Role::Ceo, None),
        ) else {
            panic!("expected an invalid outcome for {role}");
        };

        assert!(message.starts_with("role"), "{role}: {message}");
    }
}

#[test]
fn a_member_parent_outside_the_callers_subtree_is_refused() {
    let caller = AgentId::generate();
    let stranger = AgentId::generate();
    let rows = rows_of(caller, Role::Lead, None);
    let fields = create_with_role(&json!({
        "kind": "member",
        "parent_agent_id": stranger.as_uuid_text(),
    }));

    let message = refused_of(&fields, caller, &rows);

    assert!(message.contains("descendant"), "{message}");
}

#[test]
fn a_create_refuses_additions_and_any_unknown_field() {
    let caller = AgentId::generate();
    let mut fields: serde_json::Map<String, Value> = serde_json::from_value(json!({
        "name": "Bo",
        "command": "cli",
        "role": {"kind": "member"},
        "include_descendants": true,
    }))
    .expect("fields");
    fields.insert("additions".to_owned(), json!({"flags": ["--x"]}));

    // The gate refuses, it never trims: `additions` names the field at
    // fault, and so would any other key outside the allowlist.
    let error = refuse_extra_fields("agent.create", &fields).expect_err("additions are refused");
    assert!(error.contains("additions"), "{error}");

    let plain = create_with_role(&json!({"kind": "member"}));
    let allowed = allowed_fields_of(
        "agent.create",
        &plain,
        caller,
        &rows_of(caller, Role::Lead, None),
    )
    .expect("allowed");
    assert!(!allowed.contains_key("include_descendants"), "{allowed:?}");
    for key in ["name", "command", "role"] {
        assert!(allowed.contains_key(key), "{key}: {allowed:?}");
    }
}

#[test]
fn the_non_create_types_keep_only_the_agent_id() {
    let caller = AgentId::generate();
    let agent = AgentId::generate();
    let fields = serde_json::from_value(json!({
        "agent_id": agent.as_uuid_text(),
    }))
    .expect("fields");
    assert!(
        refuse_extra_fields("agent.stop", &fields).is_ok(),
        "the type's own fields pass the gate"
    );

    let fields = allowed_fields_of(
        "agent.stop",
        &fields,
        caller,
        &rows_of(caller, Role::Lead, None),
    )
    .expect("allowed");

    assert_eq!(
        fields,
        serde_json::from_value::<serde_json::Map<String, Value>>(json!({
            "agent_id": agent.as_uuid_text(),
        }))
        .expect("map")
    );
}

/// An office request with only the fields its type takes.
fn office_request(fields: &[(&str, Value)]) -> serde_json::Map<String, Value> {
    fields
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.clone()))
        .collect()
}

#[test]
fn an_office_request_with_an_extra_field_is_refused_not_dropped() {
    // A stop that smuggles `include_descendants`: dropped silently, it
    // would answer success while stopping the target alone (PR 40 LOW).
    let fields = office_request(&[("agent_id", json!(AgentId::generate().as_uuid_text()))]);
    let mut padded = fields.clone();
    padded.insert("include_descendants".to_owned(), json!(true));

    let error = refuse_extra_fields("agent.stop", &padded).expect_err("an extra field is refused");

    assert!(error.contains("include_descendants"), "{error}");
    assert!(refuse_extra_fields("agent.stop", &fields).is_ok());
}

#[test]
fn a_create_with_an_extra_field_is_refused_not_dropped() {
    let fields = office_request(&[
        ("name", json!("Bo")),
        ("command", json!("fake-cli")),
        ("unknown_thing", json!(1)),
    ]);

    let error =
        refuse_extra_fields("agent.create", &fields).expect_err("an extra field is refused");

    assert!(error.contains("unknown_thing"), "{error}");
}

#[test]
fn an_office_request_with_only_its_own_fields_passes_the_allowlist() {
    let caller = AgentId::generate();
    let target = AgentId::generate();
    let stop = office_request(&[("agent_id", json!(target.as_uuid_text()))]);
    assert!(refuse_extra_fields("agent.stop", &stop).is_ok());

    let create = office_request(&[
        ("name", json!("Bo")),
        ("command", json!("fake-cli")),
        ("role", json!({"kind": "member"})),
    ]);
    assert!(refuse_extra_fields("agent.create", &create).is_ok());
    let fields = allowed_fields_of(
        "agent.create",
        &create,
        caller,
        &rows_of(caller, Role::Lead, None),
    )
    .expect("a placed caller's create passes gate and place rules");
    assert!(fields.contains_key("role"), "{fields:?}");
}
