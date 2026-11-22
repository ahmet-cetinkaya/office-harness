// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Map, Value, json};
use template_application::common::models::ImportPreviewEntry;
use template_domain::{Template, body_to_json};
use wire::Event;

const TEMPLATE_CHANGED_EVENT: &str = "template.changed";

/// The wire form of one template (template-protocol.md).
#[must_use]
pub fn template_json(template: &Template) -> Value {
    json!({
        "id": template.id.to_string(),
        "name": template.name.as_str(),
        "kind": template.body.kind().as_str(),
        "body": body_to_json(&template.body),
        "updated_at": template.updated_at.as_unix_millis(),
    })
}

/// One template of an import preview: its name in the file, the name it will
/// be saved under, and its body with every environment value dropped.
#[must_use]
pub fn preview_json(entry: &ImportPreviewEntry) -> Value {
    json!({
        "name": entry.name.as_str(),
        "save_name": entry.save_name.as_str(),
        "kind": entry.body.kind().as_str(),
        "body": body_to_json(&entry.body),
    })
}

/// `template.changed`: the full list after any save, delete, or import
/// confirm, which clients replace their copy with.
#[must_use]
pub fn template_changed_event(templates: &[Template]) -> Event {
    let mut fields = Map::new();
    fields.insert(
        "templates".to_owned(),
        Value::Array(templates.iter().map(template_json).collect()),
    );
    Event::new(TEMPLATE_CHANGED_EVENT, fields)
}
