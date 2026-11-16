// SPDX-License-Identifier: AGPL-3.0-or-later

/// What a subscriber says about a delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryStatus {
    Accepted,
    /// The subscriber is gone or too slow; the runtime drops the subscription.
    Closed,
}
