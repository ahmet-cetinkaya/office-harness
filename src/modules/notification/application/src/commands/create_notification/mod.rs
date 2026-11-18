// SPDX-License-Identifier: AGPL-3.0-or-later

mod create_notification_handler;

pub use create_notification_handler::{CreateNotification, CreateNotificationHandler};

#[cfg(test)]
#[path = "create_notification_handler.tests.rs"]
mod tests;
