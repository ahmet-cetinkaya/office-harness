// SPDX-License-Identifier: AGPL-3.0-or-later

use notification_domain::{AttentionKind, NeedsYouItem};

/// Why an agent's item is cleared (0008 D2). A dismiss and a read of the
/// origin notification have their own service methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeedsYouClearReason {
    /// The user opened the agent's terminal (`notification.read_agent`).
    TerminalOpened,
    /// The agent went to `starting`, `working`, or `stopped`.
    StatusChanged,
    AgentDeleted,
    /// An agent hold was released. It clears an automatic hold only.
    HoldReleased,
}

impl NeedsYouClearReason {
    pub(super) const fn clears(self, item: &NeedsYouItem) -> bool {
        match self {
            Self::TerminalOpened | Self::StatusChanged | Self::AgentDeleted => true,
            Self::HoldReleased => matches!(item.latest.kind, AttentionKind::AutoHold),
        }
    }
}
