// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::{AgentId, Timestamp};
use notification_domain::{Notification, NotificationEvent, NotificationId};

use crate::common::errors::NotificationApplicationError;
use crate::common::interfaces::NotificationStore;

/// The event to record, with the agent name copied at creation.
pub struct CreateNotification {
    pub agent_id: AgentId,
    pub agent_name: String,
    pub event: NotificationEvent,
    pub at: Timestamp,
    /// See `Notification::detail`.
    pub detail: Option<String>,
}

/// Adds one notification row (FR-006). A bell while the agent still has an
/// unread bell notification adds nothing (FR-006a); every other event always
/// becomes its own row, so agents that finish together are all listed.
#[derive(Clone)]
pub struct CreateNotificationHandler {
    store: Arc<dyn NotificationStore>,
}

impl CreateNotificationHandler {
    #[must_use]
    pub fn new(store: Arc<dyn NotificationStore>) -> Self {
        Self { store }
    }

    /// The stored row, or `None` when the bell was coalesced into the unread
    /// one that is already there.
    ///
    /// # Errors
    /// `Storage`.
    pub fn handle(
        &self,
        request: CreateNotification,
    ) -> Result<Option<Notification>, NotificationApplicationError> {
        if request.event == NotificationEvent::Bell && self.has_unread_bell(request.agent_id)? {
            return Ok(None);
        }
        let mut notification = Notification::new(
            NotificationId::generate(),
            request.agent_id,
            request.agent_name,
            request.event,
            request.at,
        );
        notification.detail = request.detail;
        Ok(Some(self.store.insert(&notification)?))
    }

    fn has_unread_bell(&self, agent_id: AgentId) -> Result<bool, NotificationApplicationError> {
        Ok(self
            .store
            .list_unread_for_agent(agent_id)?
            .iter()
            .any(|notification| notification.event == NotificationEvent::Bell))
    }
}
