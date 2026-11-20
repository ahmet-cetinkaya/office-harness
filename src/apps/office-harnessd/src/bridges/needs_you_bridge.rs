// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use agent_application::common::interfaces::AgentStore;
use kernel::AgentId;
use notification_application::commands::needs_you::{NeedsYouClearReason, NeedsYouService};
use notification_application::common::errors::NotificationApplicationError;
use notification_application::common::interfaces::AgentNameLookup;
use notification_domain::{AttentionEvent, NeedsYouItem, NotificationId};
use notification_presentation::NeedsYouUseCases;

use crate::bootstrap::{ServerLog, StorageAlerts};
use crate::server::EventHub;

/// Answers the queue order's name tie-break from the agent module's store. A
/// failed or empty lookup gives an empty name: the order then falls back to
/// the wait time alone.
pub struct AgentNames {
    agents: Arc<dyn AgentStore>,
}

impl AgentNames {
    #[must_use]
    pub fn new(agents: Arc<dyn AgentStore>) -> Self {
        Self { agents }
    }
}

impl AgentNameLookup for AgentNames {
    fn name_of(&self, agent_id: AgentId) -> String {
        match self.agents.find(agent_id) {
            Ok(Some(agent)) => agent.name.as_str().to_owned(),
            _ => String::new(),
        }
    }
}

/// The one place that changes the Needs you queue and tells the clients: each
/// change that took effect is followed by `needs_you.changed` with the whole
/// ordered list (0008 contract). A failed write is reported like the
/// notification bridge reports one, and the queue stays as it was.
///
/// One turn lock covers each write, the list read after it, and the publish,
/// so a clear cannot slip between a find and a save, and two changes cannot
/// publish their lists in reverse order.
#[derive(Clone)]
pub struct NeedsYouBridge {
    service: NeedsYouService,
    agents: Arc<dyn AgentStore>,
    hub: EventHub,
    alerts: StorageAlerts,
    log: ServerLog,
    turn: Arc<Mutex<()>>,
}

impl NeedsYouBridge {
    #[must_use]
    pub fn new(
        service: NeedsYouService,
        agents: Arc<dyn AgentStore>,
        hub: EventHub,
        alerts: StorageAlerts,
        log: ServerLog,
    ) -> Self {
        Self {
            service,
            agents,
            hub,
            alerts,
            log,
            turn: Arc::new(Mutex::new(())),
        }
    }

    /// Records one attention event; `origin` is the notification it created,
    /// none for a coalesced bell. `announce_origin` runs once the item is
    /// stored and before the list goes out: a client that reads the origin on
    /// arrival then finds the item to clear (0008 D3).
    pub fn record(
        &self,
        agent_id: AgentId,
        event: AttentionEvent,
        origin: Option<NotificationId>,
        announce_origin: impl FnOnce(),
    ) {
        let _turn = self.take_turn();
        let outcome = self.service.record(agent_id, event, origin).map(|_| true);
        announce_origin();
        self.publish_if_changed("recorded", agent_id, outcome);
    }

    pub fn clear(&self, agent_id: AgentId, reason: NeedsYouClearReason) {
        let _turn = self.take_turn();
        let outcome = self.service.clear(agent_id, reason);
        self.publish_if_changed("cleared", agent_id, outcome);
    }

    /// A read of the notification that created an item clears the item.
    pub fn clear_on_read(&self, notification_id: NotificationId) {
        let _turn = self.take_turn();
        let outcome = self.service.clear_on_read(notification_id);
        self.publish_if_changed("cleared by a read", notification_id, outcome);
    }

    /// Drops the items of agents that no longer exist: after a delete, and at
    /// start for agents deleted while the server was down.
    pub fn forget_deleted_agents(&self) {
        let _turn = self.take_turn();
        let outcome = self.existing_agents().and_then(|existing| {
            self.service
                .forget_missing(&existing)
                .map_err(|error| error.to_string())
        });
        match outcome {
            Ok(removed) => self.publish_when(removed),
            Err(error) => self.report_cleanup_failure(&error),
        }
    }

    /// A failed cleanup leaves items of agents that are gone, so it raises
    /// the storage alert like the hold cleanup does.
    fn report_cleanup_failure(&self, error: &str) {
        self.alerts
            .report_disk_failure(&format!("the items of deleted agents were kept: {error}"));
    }

    fn take_turn(&self) -> MutexGuard<'_, ()> {
        self.turn.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn existing_agents(&self) -> Result<Vec<AgentId>, String> {
        let listed = self.agents.list().map_err(|error| error.to_string())?;
        Ok(listed.iter().map(|agent| agent.id).collect())
    }

    fn publish_if_changed(
        &self,
        action: &str,
        subject: impl std::fmt::Display,
        outcome: Result<bool, NotificationApplicationError>,
    ) {
        match outcome {
            Ok(changed) => self.publish_when(changed),
            Err(error) => self.report(&format!("the item of {subject} was not {action}"), &error),
        }
    }

    fn publish_when(&self, changed: bool) {
        if !changed {
            return;
        }
        match self.service.list() {
            Ok(items) => self
                .hub
                .publish(&notification_presentation::needs_you_changed_event(&items)),
            Err(error) => self.report("the needs-you list could not be read", &error),
        }
    }

    fn report(&self, what: &str, error: &NotificationApplicationError) {
        let message = format!("{what}: {error}");
        if error.is_disk_failure() {
            self.alerts.report_disk_failure(&message);
        } else {
            self.log.error(&message);
        }
    }
}

/// The protocol handlers read and dismiss through the bridge, so a dismiss
/// is followed by `needs_you.changed` like every other change.
impl NeedsYouUseCases for NeedsYouBridge {
    fn list(&self) -> Result<Vec<NeedsYouItem>, NotificationApplicationError> {
        self.service.list()
    }

    fn dismiss(
        &self,
        agent_id: AgentId,
    ) -> Result<Vec<NeedsYouItem>, NotificationApplicationError> {
        let _turn = self.take_turn();
        let items = self.service.dismiss(agent_id)?;
        self.hub
            .publish(&notification_presentation::needs_you_changed_event(&items));
        Ok(items)
    }
}

#[cfg(test)]
impl NeedsYouBridge {
    /// A queue over an in-memory database, for the bridge tests.
    pub(crate) fn in_memory(
        agents: Arc<dyn AgentStore>,
        hub: EventHub,
        alerts: StorageAlerts,
        log: ServerLog,
    ) -> Self {
        let connection = rusqlite::Connection::open_in_memory().expect("open");
        for script in notification_infrastructure::persistence::SCRIPTS {
            connection.execute_batch(script).expect("migrate");
        }
        let store = notification_infrastructure::persistence::NeedsYouRepository::new(Arc::new(
            std::sync::Mutex::new(connection),
        ));
        let service = NeedsYouService::new(
            Arc::new(store),
            Arc::new(AgentNames::new(Arc::clone(&agents))),
        );
        Self::new(service, agents, hub, alerts, log)
    }

    /// The queue as the clients see it.
    pub(crate) fn items(&self) -> Vec<notification_domain::NeedsYouItem> {
        self.service.list().expect("list")
    }
}
