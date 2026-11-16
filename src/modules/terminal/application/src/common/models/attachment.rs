// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::common::models::SubscriptionId;

/// The result of an attach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    /// Terminal bytes that redraw the scrollback, the screen, and the input
    /// modes on an empty terminal.
    pub snapshot: Vec<u8>,
    /// Set when live output follows; `None` for saved output.
    pub subscription: Option<SubscriptionId>,
}
