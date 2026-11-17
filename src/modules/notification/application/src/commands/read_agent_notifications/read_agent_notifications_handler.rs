// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::{AgentId, Timestamp};

use crate::common::errors::NotificationApplicationError;
use crate::common::interfaces::{NotificationClock, NotificationStore};

/// Marks every unread notification of one agent read (FR-009, contract
/// `notification.read_agent`). An unknown agent id is not an error: the rows
/// are the target, not the agent.
#[derive(Clone)]
pub struct ReadAgentNotificationsHandler {
    store: Arc<dyn NotificationStore>,
    clock: Arc<dyn NotificationClock>,
}

impl ReadAgentNotificationsHandler {
    #[must_use]
    pub fn new(store: Arc<dyn NotificationStore>, clock: Arc<dyn NotificationClock>) -> Self {
        Self { store, clock }
    }

    /// How many rows the read marked.
    ///
    /// # Errors
    /// `Storage`.
    pub fn handle(&self, agent_id: AgentId) -> Result<usize, NotificationApplicationError> {
        let at: Timestamp = self.clock.now();
        let mut marked = 0;
        for mut notification in self.store.list_unread_for_agent(agent_id)? {
            notification.mark_read(at);
            self.store.update(&notification)?;
            marked += 1;
        }
        Ok(marked)
    }
}
