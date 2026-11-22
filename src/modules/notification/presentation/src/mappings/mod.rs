// SPDX-License-Identifier: AGPL-3.0-or-later

mod notification_json;

pub use notification_json::{
    needs_you_changed_event, needs_you_item_json, notification_created_event, notification_json,
};

#[cfg(test)]
#[path = "notification_json.tests.rs"]
mod tests;
