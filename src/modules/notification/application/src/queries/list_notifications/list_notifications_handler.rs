// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use notification_domain::Notification;

use crate::common::errors::NotificationApplicationError;
use crate::common::interfaces::NotificationStore;

/// At most this many rows per call (contract `notification.list`, D4).
pub const LIST_LIMIT: usize = 500;

/// The rows for the panel's notification list: unread first, then newest
/// first, capped at `LIST_LIMIT`.
#[derive(Clone)]
pub struct ListNotificationsHandler {
    store: Arc<dyn NotificationStore>,
}

impl ListNotificationsHandler {
    #[must_use]
    pub fn new(store: Arc<dyn NotificationStore>) -> Self {
        Self { store }
    }

    /// The rows for the panel's list: unread first, then newest first, capped
    /// in the store.
    ///
    /// # Errors
    /// `Storage`.
    pub fn handle(&self) -> Result<Vec<Notification>, NotificationApplicationError> {
        self.store.list()
    }

    /// The same rows without the read ones.
    ///
    /// # Errors
    /// `Storage`.
    pub fn handle_unread(&self) -> Result<Vec<Notification>, NotificationApplicationError> {
        self.store.list_unread()
    }
}
