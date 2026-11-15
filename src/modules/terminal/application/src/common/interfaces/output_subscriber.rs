// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::common::models::{DeliveryStatus, LiveOutput};

/// Receives live output of one terminal. The runtime calls `deliver` while it
/// holds the terminal's lock, which keeps the snapshot and the live stream
/// free of gaps and repeats. An implementation must therefore return at once
/// and never block, for example by pushing into a bounded channel.
pub trait OutputSubscriber: Send + Sync {
    fn deliver(&self, output: LiveOutput) -> DeliveryStatus;
}
