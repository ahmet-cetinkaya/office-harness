// SPDX-License-Identifier: AGPL-3.0-or-later

use notification_domain::{NeedsYouItem, Notification};
use serde_json::{Map, Value, json};
use wire::Event;

/// The wire form of one notification (contract `notification.list`).
#[must_use]
pub fn notification_json(notification: &Notification) -> Value {
    json!({
        "id": notification.id.as_uuid_text(),
        "agent_id": notification.agent_id.map(|agent_id| agent_id.to_string()),
        "agent_name": notification.agent_name,
        "event": notification.event.as_str(),
        "created_at": notification.created_at.as_unix_millis(),
        "read_at": notification
            .read_at
            .map(|at| json!(at.as_unix_millis())),
        "detail": notification.detail,
    })
}

/// `notification.created`, sent after a row is created (D4, D5).
#[must_use]
pub fn notification_created_event(notification: &Notification) -> Event {
    let mut fields = Map::new();
    fields.insert("notification".to_owned(), notification_json(notification));
    Event::new("notification.created", fields)
}

/// The wire form of one needs-you item (contract `needs_you.list`).
#[must_use]
pub fn needs_you_item_json(item: &NeedsYouItem) -> Value {
    json!({
        "agent_id": item.agent_id.to_string(),
        "kind": item.latest.kind.as_str(),
        "detail": item.latest.detail,
        "first_event_at": item.first_event_at.as_unix_millis(),
        "latest_at": item.latest.at.as_unix_millis(),
        "origin_notification_id": item.origin.map(|origin| origin.as_uuid_text()),
        "report": item.report.as_ref().map(|report| json!({
            "kind": report.kind,
            "text": report.text,
            "at": report.at.as_unix_millis(),
        })),
    })
}

/// `needs_you.changed`: the full ordered list after any change (contract).
#[must_use]
pub fn needs_you_changed_event(items: &[NeedsYouItem]) -> Event {
    let mut fields = Map::new();
    fields.insert(
        "items".to_owned(),
        Value::Array(items.iter().map(needs_you_item_json).collect()),
    );
    Event::new("needs_you.changed", fields)
}
