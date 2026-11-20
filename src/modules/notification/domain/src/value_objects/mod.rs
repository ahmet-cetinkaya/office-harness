// SPDX-License-Identifier: AGPL-3.0-or-later

mod attention_event;
mod notification_id;

pub use attention_event::{AttentionEvent, AttentionKind};
pub use notification_id::NotificationId;

#[cfg(test)]
#[path = "attention_event.tests.rs"]
mod attention_event_tests;
