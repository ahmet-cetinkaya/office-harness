// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::Timestamp;

use super::*;

const SINCE: i64 = 1_700_000_000_000;

#[test]
fn the_office_hold_has_no_agent_id_and_a_user_hold_has_no_reason() {
    let body = hold_json(&Hold {
        scope: HoldScope::Office,
        set_by: HoldSetter::User,
        since: Timestamp::from_unix_millis(SINCE),
        reason: None,
    });

    assert_eq!(body, json!({"set_by": "user", "since": SINCE}));
}

#[test]
fn an_automatic_agent_hold_names_the_agent_and_the_reason() {
    let agent = AgentId::generate();
    let body = hold_json(&Hold {
        scope: HoldScope::Agent(agent),
        set_by: HoldSetter::Office,
        since: Timestamp::from_unix_millis(SINCE),
        reason: Some("the limit".to_owned()),
    });

    assert_eq!(
        body,
        json!({
            "agent_id": agent.to_string(),
            "set_by": "office",
            "reason": "the limit",
            "since": SINCE,
        })
    );
}

#[test]
fn hold_changed_carries_the_office_hold_or_null_and_the_agent_holds() {
    let event = hold_changed_event(&HoldsView::default()).encode();

    let body: Value = serde_json::from_str(&event).expect("json");
    assert_eq!(body["type"], "hold.changed");
    assert_eq!(body["office"], Value::Null);
    assert_eq!(body["agents"], json!([]));
}

#[test]
fn errors_map_to_the_contract_codes_and_a_conflict_keeps_its_text_alone() {
    let conflict = HierarchyApplicationError::Conflict("the office is already on hold".to_owned());
    assert_eq!(error_code(&conflict), ErrorCode::Conflict);
    assert_eq!(message_of(&conflict), "the office is already on hold");
    assert_eq!(
        error_code(&HierarchyApplicationError::NotFound("x".to_owned())),
        ErrorCode::NotFound
    );
    assert_eq!(
        error_code(&HierarchyApplicationError::Storage("x".to_owned())),
        ErrorCode::Internal
    );
}

#[test]
fn only_the_four_hold_requests_are_accepted() {
    for kind in ["hold.set", "hold.release", "hold.list", "office.actions"] {
        assert!(HoldGateway::accepts(kind), "{kind}");
    }
    assert!(!HoldGateway::accepts("hold.other"));
    assert!(!HoldGateway::accepts("agent.list"));
}
