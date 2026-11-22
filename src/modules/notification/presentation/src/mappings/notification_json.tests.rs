// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::{AgentId, Timestamp};
use notification_domain::{
    AttentionEvent, AttentionKind, NeedsYouItem, Notification, NotificationEvent, NotificationId,
};
use serde_json::Value;

use super::*;

fn stored(event: NotificationEvent, read_at: Option<Timestamp>) -> Notification {
    Notification {
        id: NotificationId::from_uuid_text("00000000-0000-0000-0000-000000000001")
            .expect("fixed id"),
        agent_id: AgentId::from_u128(2),
        agent_name: "Ana".to_owned(),
        event,
        created_at: Timestamp::from_unix_millis(1_000),
        read_at,
        detail: None,
    }
}

#[test]
fn a_notification_is_listed_with_every_contract_field() {
    let body = notification_json(&stored(NotificationEvent::Exited, None));

    assert_eq!(body["id"], "00000000-0000-0000-0000-000000000001");
    assert_eq!(body["agent_id"], AgentId::from_u128(2).to_string());
    assert_eq!(body["agent_name"], "Ana");
    assert_eq!(body["event"], "exited");
    assert_eq!(body["created_at"], 1_000);
    assert_eq!(body["read_at"], Value::Null);
    assert_eq!(body["detail"], Value::Null);
}

#[test]
fn an_auto_hold_carries_its_detail() {
    let row = stored(NotificationEvent::AutoHold, None).with_detail("a limit".to_owned());

    let body = notification_json(&row);

    assert_eq!(body["event"], "auto_hold");
    assert_eq!(body["detail"], "a limit");
}

#[test]
fn a_read_notification_carries_its_read_time_and_a_bell_its_name() {
    let body = notification_json(&stored(
        NotificationEvent::Bell,
        Some(Timestamp::from_unix_millis(2_000)),
    ));

    assert_eq!(body["event"], "bell");
    assert_eq!(body["read_at"], 2_000);
}

#[test]
fn the_created_event_names_the_notification_object() {
    let event = notification_created_event(&stored(NotificationEvent::WaitingForInput, None));
    let body: Value = serde_json::from_str(&event.encode()).expect("event body");

    assert_eq!(body["type"], "notification.created");
    assert_eq!(body["notification"]["event"], "waiting_for_input");
    assert_eq!(body["notification"]["agent_name"], "Ana");
}

fn item_with_origin(origin: Option<NotificationId>) -> NeedsYouItem {
    let at = Timestamp::from_unix_millis(1_000);
    NeedsYouItem {
        agent_id: AgentId::from_u128(2),
        first_event_at: at,
        latest: AttentionEvent {
            kind: AttentionKind::Bell,
            detail: None,
            at,
        },
        origin,
    }
}

#[test]
fn an_item_names_its_origin_notification_or_sends_null() {
    let origin =
        NotificationId::from_uuid_text("00000000-0000-0000-0000-000000000001").expect("fixed id");

    let named = needs_you_item_json(&item_with_origin(Some(origin)));
    let coalesced = needs_you_item_json(&item_with_origin(None));

    assert_eq!(
        named["origin_notification_id"],
        "00000000-0000-0000-0000-000000000001"
    );
    assert_eq!(coalesced["origin_notification_id"], Value::Null);
    assert_eq!(coalesced["detail"], Value::Null);
}
