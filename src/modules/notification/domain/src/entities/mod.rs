// SPDX-License-Identifier: AGPL-3.0-or-later

mod notification;

pub use notification::Notification;

#[cfg(test)]
#[path = "notification.tests.rs"]
mod tests;
