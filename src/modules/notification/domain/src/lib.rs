// SPDX-License-Identifier: AGPL-3.0-or-later

//! Notification module domain: one row per event that the user must see.
//! No I/O.

mod entities;
mod enums;
mod errors;
mod value_objects;

pub use entities::{NeedsYouItem, Notification, order, record_event};
pub use enums::NotificationEvent;
pub use errors::NotificationError;
pub use value_objects::{AttentionEvent, AttentionKind, NotificationId};
