// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;
use notification_domain::{NeedsYouItem, NotificationId};

use crate::common::errors::NotificationApplicationError;

/// Where the needs-you items live: one per agent.
pub trait NeedsYouStore: Send + Sync {
    /// The agent's item, if it has one.
    ///
    /// # Errors
    /// `Storage` when the row cannot be read.
    fn find(&self, agent_id: AgentId)
    -> Result<Option<NeedsYouItem>, NotificationApplicationError>;

    /// Stores the item, replacing the agent's old one.
    ///
    /// # Errors
    /// `Storage` when the row cannot be written.
    fn save(&self, item: &NeedsYouItem) -> Result<(), NotificationApplicationError>;

    /// Every item, in no particular order.
    ///
    /// # Errors
    /// `Storage` when the rows cannot be read.
    fn list(&self) -> Result<Vec<NeedsYouItem>, NotificationApplicationError>;

    /// Removes the agent's item. True when there was one.
    ///
    /// # Errors
    /// `Storage` when the row cannot be written.
    fn remove(&self, agent_id: AgentId) -> Result<bool, NotificationApplicationError>;

    /// Removes the item that the notification created. True when there was
    /// one.
    ///
    /// # Errors
    /// `Storage` when the row cannot be written.
    fn remove_by_origin(
        &self,
        notification_id: NotificationId,
    ) -> Result<bool, NotificationApplicationError>;
}
