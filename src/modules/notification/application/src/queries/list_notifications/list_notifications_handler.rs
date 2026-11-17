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

    /// `unread_only` keeps only the rows with no read time.
    ///
    /// # Errors
    /// `Storage`.
    pub fn handle(
        &self,
        unread_only: bool,
    ) -> Result<Vec<Notification>, NotificationApplicationError> {
        let rows = self.store.list()?;
        let rows = rows
            .into_iter()
            .filter(|notification| !unread_only || notification.is_unread())
            .take(LIST_LIMIT)
            .collect();
        Ok(rows)
    }
}
