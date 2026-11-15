// SPDX-License-Identifier: AGPL-3.0-or-later

/// Names one subscription of a terminal, so a client can detach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(pub u64);
