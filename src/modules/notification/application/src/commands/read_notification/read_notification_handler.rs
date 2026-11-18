// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use notification_domain::NotificationId;

use crate::common::errors::NotificationApplicationError;
use crate::common::interfaces::{NotificationClock, NotificationStore};

/// Marks one notification read (FR-009, contract `notification.read`).
#[derive(Clone)]
pub struct ReadNotificationHandler {
    store: Arc<dyn NotificationStore>,
    clock: Arc<dyn NotificationClock>,
}

impl ReadNotificationHandler {
    #[must_use]
    pub fn new(store: Arc<dyn NotificationStore>, clock: Arc<dyn NotificationClock>) -> Self {
        Self { store, clock }
    }

    /// # Errors
    /// `NotFound` when the id is unknown, `Storage` when the row cannot be
    /// read or written.
    pub fn handle(
        &self,
        notification_id: NotificationId,
    ) -> Result<(), NotificationApplicationError> {
        let mut notification = self.store.find(notification_id)?.ok_or_else(|| {
            NotificationApplicationError::NotFound(format!(
                "notification {notification_id} was not found"
            ))
        })?;
        notification.mark_read(self.clock.now());
        self.store.update(&notification)
    }
}
