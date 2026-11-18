// SPDX-License-Identifier: AGPL-3.0-or-later

mod list_notifications_handler;

pub use list_notifications_handler::{LIST_LIMIT, ListNotificationsHandler};

#[cfg(test)]
#[path = "list_notifications_handler.tests.rs"]
mod tests;
