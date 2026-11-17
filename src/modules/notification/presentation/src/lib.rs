// SPDX-License-Identifier: AGPL-3.0-or-later

//! Notification module protocol surface: the `notification.*` requests and
//! the `notification.created` event.

pub mod contracts;
pub mod handlers;
pub mod mappings;

#[cfg(test)]
mod test_support;

pub use contracts::{ListNotificationsRequest, NotificationIdRequest, ReadAgentRequest};
pub use handlers::{NotificationHandlers, NotificationUseCases, REQUEST_PREFIX};
pub use mappings::{notification_created_event, notification_json};
