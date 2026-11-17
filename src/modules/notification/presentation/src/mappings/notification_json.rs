// SPDX-License-Identifier: AGPL-3.0-or-later

use notification_domain::Notification;
use serde_json::{Map, Value, json};
use wire::Event;

/// The wire form of one notification (contract `notification.list`).
#[must_use]
pub fn notification_json(notification: &Notification) -> Value {
    json!({
        "id": notification.id.as_uuid_text(),
        "agent_id": notification.agent_id.to_string(),
        "agent_name": notification.agent_name,
        "event": notification.event.as_str(),
        "created_at": notification.created_at.as_unix_millis(),
        "read_at": notification
            .read_at
            .map(|at| json!(at.as_unix_millis())),
    })
}

/// `notification.created`, sent after a row is created (D4, D5).
#[must_use]
pub fn notification_created_event(notification: &Notification) -> Event {
    let mut fields = Map::new();
    fields.insert("notification".to_owned(), notification_json(notification));
    Event::new("notification.created", fields)
}
