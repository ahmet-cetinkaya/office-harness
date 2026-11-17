// SPDX-License-Identifier: AGPL-3.0-or-later

//! In-memory fake of the store port, shared by the unit tests.

use std::sync::Mutex;

use kernel::{AgentId, Timestamp};
use notification_domain::{Notification, NotificationEvent, NotificationId};

use crate::common::errors::NotificationApplicationError;
use crate::common::interfaces::{NotificationClock, NotificationStore};

/// The clock the tests stamp reads with.
pub struct TestClock(pub Timestamp);

impl NotificationClock for TestClock {
    fn now(&self) -> Timestamp {
        self.0
    }
}

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Keeps the rows the way the listing orders them: unread first, then newest
/// first. Inserts carry their own created time, so the order is by it.
#[derive(Default)]
pub struct MemoryNotifications(Mutex<Vec<Notification>>);

impl MemoryNotifications {
    #[must_use]
    pub fn all(&self) -> Vec<Notification> {
        locked(&self.0).clone()
    }

    /// Seeds a row as it is, read time included, for tests that arrange a
    /// specific stored state.
    pub fn seed(&self, notification: Notification) {
        locked(&self.0).push(notification);
    }
}

fn newest_first(left: &Notification, right: &Notification) -> std::cmp::Ordering {
    right
        .created_at
        .cmp(&left.created_at)
        .then_with(|| left.id.as_uuid_text().cmp(&right.id.as_uuid_text()))
}

fn unread_first_then_newest(rows: &mut [Notification]) {
    rows.sort_by(|left, right| {
        let left_unread = u8::from(left.is_unread());
        let right_unread = u8::from(right.is_unread());
        right_unread
            .cmp(&left_unread)
            .then_with(|| newest_first(left, right))
    });
}

impl NotificationStore for MemoryNotifications {
    fn insert(
        &self,
        notification: &Notification,
    ) -> Result<Notification, NotificationApplicationError> {
        let mut rows = locked(&self.0);
        rows.push(notification.clone());
        Ok(notification.clone())
    }

    fn list(&self) -> Result<Vec<Notification>, NotificationApplicationError> {
        let mut rows = locked(&self.0).clone();
        unread_first_then_newest(&mut rows);
        Ok(rows)
    }

    fn find(
        &self,
        notification_id: NotificationId,
    ) -> Result<Option<Notification>, NotificationApplicationError> {
        Ok(locked(&self.0)
            .iter()
            .find(|notification| notification.id == notification_id)
            .cloned())
    }

    fn update(&self, notification: &Notification) -> Result<(), NotificationApplicationError> {
        let mut rows = locked(&self.0);
        if let Some(stored) = rows.iter_mut().find(|stored| stored.id == notification.id) {
            *stored = notification.clone();
        }
        Ok(())
    }

    fn list_unread_for_agent(
        &self,
        agent_id: AgentId,
    ) -> Result<Vec<Notification>, NotificationApplicationError> {
        Ok(locked(&self.0)
            .iter()
            .filter(|notification| notification.agent_id == agent_id && notification.is_unread())
            .cloned()
            .collect())
    }
}

/// A notification of one agent with the given event and created time.
#[must_use]
pub fn notification_of(
    agent_id: AgentId,
    name: &str,
    event: NotificationEvent,
    created_at: i64,
) -> Notification {
    Notification::new(
        NotificationId::generate(),
        agent_id,
        name.to_owned(),
        event,
        Timestamp::from_unix_millis(created_at),
    )
}
