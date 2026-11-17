// SPDX-License-Identifier: AGPL-3.0-or-later

mod read_notification_handler;

pub use read_notification_handler::ReadNotificationHandler;

#[cfg(test)]
#[path = "read_notification_handler.tests.rs"]
mod tests;
