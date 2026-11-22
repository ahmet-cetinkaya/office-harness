// SPDX-License-Identifier: AGPL-3.0-or-later

mod needs_you_item;
mod notification;

pub use needs_you_item::{NeedsYouItem, order, record_event};
pub use notification::Notification;

#[cfg(test)]
#[path = "needs_you_item.tests.rs"]
mod needs_you_item_tests;
#[cfg(test)]
#[path = "notification.tests.rs"]
mod tests;
