// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::entities::Session;

/// A session with the answer to "can the user continue it?" (FR-025).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionView {
    pub session: Session,
    /// Why the session cannot be continued; `None` when it can.
    pub cannot_continue_reason: Option<String>,
}

impl SessionView {
    #[must_use]
    pub fn can_continue(&self) -> bool {
        self.cannot_continue_reason.is_none()
    }
}
