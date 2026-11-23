// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::{AgentId, Timestamp};
use notification_domain::{Notification, NotificationEvent, NotificationId};

use crate::common::errors::NotificationApplicationError;
use crate::common::interfaces::NotificationStore;

/// The event to record, with the agent name copied at creation.
pub struct CreateNotification {
    /// See `Notification::agent_id`.
    pub agent_id: Option<AgentId>,
    pub agent_name: String,
    pub event: NotificationEvent,
    pub at: Timestamp,
    /// See `Notification::detail`.
    pub detail: Option<String>,
}

/// The events that add no row while an unread row of the same event and agent
/// exists: a bell (0003/FR-006 (a)) and a report for the person (0015).
const COALESCED_EVENTS: [NotificationEvent; 2] =
    [NotificationEvent::Bell, NotificationEvent::Report];

/// Adds one notification row (FR-006). A bell or a report while the agent
/// still has an unread one of the same event adds nothing (FR-006a, as 0015
/// modified it); every other event always becomes its own row, so agents
/// that finish together are all listed.
#[derive(Clone)]
pub struct CreateNotificationHandler {
    store: Arc<dyn NotificationStore>,
}

impl CreateNotificationHandler {
    #[must_use]
    pub fn new(store: Arc<dyn NotificationStore>) -> Self {
        Self { store }
    }

    /// The stored row, or `None` when a bell or a report was coalesced into
    /// the unread one of the same event that is already there (0003/FR-006
    /// (a), as 0015 modified it).
    ///
    /// # Errors
    /// `Storage`.
    pub fn handle(
        &self,
        request: CreateNotification,
    ) -> Result<Option<Notification>, NotificationApplicationError> {
        if let Some(agent_id) = request.agent_id
            && COALESCED_EVENTS.contains(&request.event)
            && self.has_unread(agent_id, request.event)?
        {
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

    fn has_unread(
        &self,
        agent_id: AgentId,
        event: NotificationEvent,
    ) -> Result<bool, NotificationApplicationError> {
        Ok(self
            .store
            .list_unread_for_agent(agent_id)?
            .iter()
            .any(|notification| notification.event == event))
    }
}
