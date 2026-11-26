// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::{AgentId, Timestamp};

use crate::enums::NotificationEvent;
use crate::value_objects::NotificationId;

/// One event that the user must see. The agent name is copied at creation, so
/// the row stays readable after the agent is deleted (spec edge case); for a
/// row without an agent it is the text to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub id: NotificationId,
    /// `None` for a notification that no agent caused, such as the missed
    /// runs of schedules (0025 D14).
    pub agent_id: Option<AgentId>,
    pub agent_name: String,
    pub event: NotificationEvent,
    pub created_at: Timestamp,
    /// `None` while unread (FR-007, FR-009).
    pub read_at: Option<Timestamp>,
    /// What the event adds to its name: the limit that held the agent for
    /// `auto_hold` (0007 D7); `None` for the four 0003 events.
    pub detail: Option<String>,
}

impl Notification {
    /// A new, unread notification.
    #[must_use]
    pub fn new(
        id: NotificationId,
        agent_id: Option<AgentId>,
        agent_name: String,
        event: NotificationEvent,
        created_at: Timestamp,
    ) -> Self {
        Self {
            id,
            agent_id,
            agent_name,
            event,
            created_at,
            read_at: None,
            detail: None,
        }
    }

    #[must_use]
    pub fn with_detail(mut self, detail: String) -> Self {
        self.detail = Some(detail);
        self
    }

    #[must_use]
    pub fn is_unread(&self) -> bool {
        self.read_at.is_none()
    }

    /// Marks it read; an already-read notification keeps its first read time.
    pub fn mark_read(&mut self, at: Timestamp) {
        if self.read_at.is_none() {
            self.read_at = Some(at);
        }
    }
}
