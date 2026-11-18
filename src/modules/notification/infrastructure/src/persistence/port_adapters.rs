// SPDX-License-Identifier: AGPL-3.0-or-later

//! The repository as the store that the notification use cases declare.

use kernel::AgentId;
use notification_application::common::errors::NotificationApplicationError;
use notification_application::common::interfaces::NotificationStore;
use notification_domain::{Notification, NotificationId};

use crate::persistence::NotificationRepository;

impl NotificationStore for NotificationRepository {
    fn insert(
        &self,
        notification: &Notification,
    ) -> Result<Notification, NotificationApplicationError> {
        Self::insert(self, notification)?;
        Ok(notification.clone())
    }

    fn list(&self) -> Result<Vec<Notification>, NotificationApplicationError> {
        Self::list(self)
    }

    fn find(
        &self,
        notification_id: NotificationId,
    ) -> Result<Option<Notification>, NotificationApplicationError> {
        Self::find(self, notification_id)
    }

    fn update(&self, notification: &Notification) -> Result<(), NotificationApplicationError> {
        Self::update(self, notification)
    }

    fn list_unread_for_agent(
        &self,
        agent_id: AgentId,
    ) -> Result<Vec<Notification>, NotificationApplicationError> {
        Self::list_unread_for_agent(self, agent_id)
    }
}
