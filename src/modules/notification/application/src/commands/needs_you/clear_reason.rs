// SPDX-License-Identifier: AGPL-3.0-or-later

use notification_domain::{AttentionKind, NeedsYouItem};

/// Why an agent's item is cleared (0008 D2). A dismiss and a read of the
/// origin notification have their own service methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeedsYouClearReason {
    /// The user opened the agent's terminal (`notification.read_agent`).
    TerminalOpened,
    /// The agent went to `starting` or `working`. It leaves an item that
    /// holds a report, which the user has not read yet (0015/FR-008).
    Resumed,
    /// The agent went to `stopped`.
    Stopped,
    AgentDeleted,
    /// An agent hold was released. It clears an automatic hold and the hold of
    /// a failed resume only, and never an item that holds a report.
    HoldReleased,
}

impl NeedsYouClearReason {
    pub(super) const fn clears(self, item: &NeedsYouItem) -> bool {
        match self {
            Self::TerminalOpened | Self::Stopped | Self::AgentDeleted => true,
            Self::Resumed => item.report.is_none(),
            Self::HoldReleased => {
                item.report.is_none()
                    && matches!(
                        item.latest.kind,
                        AttentionKind::AutoHold | AttentionKind::ResumeHold
                    )
            }
        }
    }
}
