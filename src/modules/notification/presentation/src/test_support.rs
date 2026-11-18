// SPDX-License-Identifier: AGPL-3.0-or-later

//! In-memory fakes for the handler tests.

use std::sync::Mutex;

use kernel::Timestamp;
use notification_application::common::errors::NotificationApplicationError;
use notification_application::common::interfaces::{NotificationClock, NotificationStore};
use notification_application::queries::list_notifications::LIST_LIMIT;
use notification_domain::{Notification, NotificationId};

pub struct TestClock(pub Timestamp);

impl NotificationClock for TestClock {
    fn now(&self) -> Timestamp {
        self.0
    }
}

#[derive(Default)]
pub struct MemoryNotifications(Mutex<Vec<Notification>>);

/// The rows the way the store contract hands them out: unread first, then
/// newest first.
fn unread_first_then_newest(rows: &mut [Notification]) {
    rows.sort_by(|left, right| {
        let left_unread = u8::from(left.is_unread());
        let right_unread = u8::from(right.is_unread());
        right_unread
            .cmp(&left_unread)
            .then_with(|| right.created_at.cmp(&left.created_at))
    });
}

impl MemoryNotifications {
    /// Stores one row the way the use case does.
    pub fn push(&self, notification: &Notification) {
        self.0.lock().expect("lock").push(notification.clone());
    }

    /// The rows, unread first, then newest first.
    #[must_use]
    pub fn list(&self) -> Vec<Notification> {
        let mut rows = self.0.lock().expect("lock").clone();
        unread_first_then_newest(&mut rows);
        rows
    }

    /// Marks one row read, so a test arranges a stored read state.
    pub fn with_read(&self, notification_id: NotificationId, at: Timestamp) {
        let mut rows = self.0.lock().expect("lock");
        if let Some(row) = rows
            .iter_mut()
            .find(|row| row.id == notification_id && row.read_at.is_none())
        {
            row.mark_read(at);
        }
    }
}

impl NotificationStore for MemoryNotifications {
    fn insert(
        &self,
        notification: &Notification,
    ) -> Result<Notification, NotificationApplicationError> {
        self.0.lock().expect("lock").push(notification.clone());
        Ok(notification.clone())
    }

    fn list(&self) -> Result<Vec<Notification>, NotificationApplicationError> {
        let mut rows = self.0.lock().expect("lock").clone();
        unread_first_then_newest(&mut rows);
        rows.truncate(LIST_LIMIT);
        Ok(rows)
    }

    fn list_unread(&self) -> Result<Vec<Notification>, NotificationApplicationError> {
        let mut rows: Vec<Notification> = self
            .0
            .lock()
            .expect("lock")
            .iter()
            .filter(|row| row.is_unread())
            .cloned()
            .collect();
        unread_first_then_newest(&mut rows);
        rows.truncate(LIST_LIMIT);
        Ok(rows)
    }

    fn find(
        &self,
        notification_id: NotificationId,
    ) -> Result<Option<Notification>, NotificationApplicationError> {
        Ok(self
            .0
            .lock()
            .expect("lock")
            .iter()
            .find(|row| row.id == notification_id)
            .cloned())
    }

    fn update(&self, notification: &Notification) -> Result<(), NotificationApplicationError> {
        let mut rows = self.0.lock().expect("lock");
        if let Some(row) = rows.iter_mut().find(|row| row.id == notification.id) {
            *row = notification.clone();
        }
        Ok(())
    }

    fn list_unread_for_agent(
        &self,
        agent_id: kernel::AgentId,
    ) -> Result<Vec<Notification>, NotificationApplicationError> {
        Ok(self
            .0
            .lock()
            .expect("lock")
            .iter()
            .filter(|row| row.agent_id == agent_id && row.is_unread())
            .cloned()
            .collect())
    }
}
