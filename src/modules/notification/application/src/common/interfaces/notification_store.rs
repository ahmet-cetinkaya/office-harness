// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;
use notification_domain::Notification;

use crate::common::errors::NotificationApplicationError;

/// Where the notifications live.
pub trait NotificationStore: Send + Sync {
    /// Stores the notification and returns the stored row.
    ///
    /// # Errors
    /// `Storage` when the row cannot be written.
    fn insert(
        &self,
        notification: &Notification,
    ) -> Result<Notification, NotificationApplicationError>;

    /// The newest rows, unread first, then newest first, at most
    /// [`crate::queries::list_notifications::LIST_LIMIT`] of them.
    ///
    /// # Errors
    /// `Storage` when the rows cannot be read.
    fn list(&self) -> Result<Vec<Notification>, NotificationApplicationError>;

    /// The newest unread rows, newest first, at most
    /// [`crate::queries::list_notifications::LIST_LIMIT`] of them.
    ///
    /// # Errors
    /// `Storage` when the rows cannot be read.
    fn list_unread(&self) -> Result<Vec<Notification>, NotificationApplicationError>;

    /// The row with the id, if it is still there.
    ///
    /// # Errors
    /// `Storage` when the rows cannot be read.
    fn find(
        &self,
        notification_id: notification_domain::NotificationId,
    ) -> Result<Option<Notification>, NotificationApplicationError>;

    /// Stores the row with its new read time.
    ///
    /// # Errors
    /// `Storage` when the row cannot be written.
    fn update(&self, notification: &Notification) -> Result<(), NotificationApplicationError>;

    /// The unread rows of one agent.
    ///
    /// # Errors
    /// `Storage` when the rows cannot be read.
    fn list_unread_for_agent(
        &self,
        agent_id: AgentId,
    ) -> Result<Vec<Notification>, NotificationApplicationError>;
}
