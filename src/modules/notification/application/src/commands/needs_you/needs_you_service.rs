// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;
use std::sync::Arc;

use kernel::AgentId;
use notification_domain::{AttentionEvent, NeedsYouItem, NotificationId, order, record_event};

use super::NeedsYouClearReason;
use crate::common::errors::NotificationApplicationError;
use crate::common::interfaces::{AgentNameLookup, NeedsYouStore};

/// Keeps one needs-you item per agent (0008 D1, D2).
///
/// The clearing methods return true when an item went away, so the caller
/// knows whether to publish the changed list.
#[derive(Clone)]
pub struct NeedsYouService {
    store: Arc<dyn NeedsYouStore>,
    names: Arc<dyn AgentNameLookup>,
}

impl NeedsYouService {
    #[must_use]
    pub fn new(store: Arc<dyn NeedsYouStore>, names: Arc<dyn AgentNameLookup>) -> Self {
        Self { store, names }
    }

    /// Records an attention event: a new item, or a new latest event on the
    /// agent's item.
    ///
    /// # Errors
    /// `Storage`.
    pub fn record(
        &self,
        agent_id: AgentId,
        event: AttentionEvent,
        origin: Option<NotificationId>,
    ) -> Result<NeedsYouItem, NotificationApplicationError> {
        let existing = self.store.find(agent_id)?;
        let item = record_event(existing, agent_id, event, origin);
        self.store.save(&item)?;
        Ok(item)
    }

    /// Clears the agent's item when the reason applies to it.
    ///
    /// # Errors
    /// `Storage`.
    pub fn clear(
        &self,
        agent_id: AgentId,
        reason: NeedsYouClearReason,
    ) -> Result<bool, NotificationApplicationError> {
        match self.store.find(agent_id)? {
            Some(item) if reason.clears(&item) => self.store.remove(agent_id),
            _ => Ok(false),
        }
    }

    /// Clears the item that the notification created, as its read does.
    ///
    /// # Errors
    /// `Storage`.
    pub fn clear_on_read(
        &self,
        notification_id: NotificationId,
    ) -> Result<bool, NotificationApplicationError> {
        self.store.remove_by_origin(notification_id)
    }

    /// Removes the agent's item on the user's request and returns the list
    /// that is left. Nothing is marked read.
    ///
    /// # Errors
    /// `NotFound` when the agent has no item, `Storage`.
    pub fn dismiss(
        &self,
        agent_id: AgentId,
    ) -> Result<Vec<NeedsYouItem>, NotificationApplicationError> {
        if !self.store.remove(agent_id)? {
            return Err(NotificationApplicationError::NotFound(format!(
                "agent {agent_id} has no needs-you item"
            )));
        }
        self.list()
    }

    /// Every item in the queue order.
    ///
    /// # Errors
    /// `Storage`.
    pub fn list(&self) -> Result<Vec<NeedsYouItem>, NotificationApplicationError> {
        let items = self.store.list()?;
        Ok(order(items, |agent_id| self.names.name_of(agent_id)))
    }

    /// Drops the items of agents that no longer exist, for the ones deleted
    /// while the server was down. True when an item went away.
    ///
    /// # Errors
    /// `Storage`.
    pub fn forget_missing(
        &self,
        existing_agents: &[AgentId],
    ) -> Result<bool, NotificationApplicationError> {
        let existing: HashSet<AgentId> = existing_agents.iter().copied().collect();
        let mut removed_any = false;
        for item in self.store.list()? {
            if !existing.contains(&item.agent_id) {
                removed_any |= self.store.remove(item.agent_id)?;
            }
        }
        Ok(removed_any)
    }
}
