// SPDX-License-Identifier: AGPL-3.0-or-later

//! Notification module protocol surface: the `notification.*` and
//! `needs_you.*` requests and the `notification.created` and
//! `needs_you.changed` events.

pub mod contracts;
pub mod handlers;
pub mod mappings;

#[cfg(test)]
mod test_support;

pub use contracts::{
    DismissNeedsYouRequest, ListNeedsYouRequest, ListNotificationsRequest, NotificationIdRequest,
    ReadAgentRequest,
};
pub use handlers::{
    NEEDS_YOU_REQUEST_PREFIX, NeedsYouHandlers, NeedsYouUseCases, NotificationHandlers,
    NotificationUseCases, REQUEST_PREFIX,
};
pub use mappings::{
    needs_you_changed_event, needs_you_item_json, notification_created_event, notification_json,
};
