// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use audit_application::commands::record_action::RecordActionHandler;
use audit_domain::{ActionKind, ActionRecord, ActionResult};
use audit_presentation::action_recorded_event;
use hierarchy_application::HoldsView;
use hierarchy_application::common::interfaces::HoldStore;
use hierarchy_application::common::ports::{HoldJournal, HoldNotifier};
use hierarchy_domain::{Hold, HoldScope, HoldSetter, LimitRefusal};
use kernel::{Actor, AgentId, Timestamp};
use notification_application::commands::needs_you::NeedsYouClearReason;

use super::{NeedsYouBridge, NotificationBridge, SystemClock};
use crate::bootstrap::StorageAlerts;
use crate::server::{EventHub, hold_changed_event};
use agent_application::common::interfaces::Clock;

/// Writes hold, release, and automatic hold into the action record (FR-010)
/// and tells the clients with `action.recorded`. A user hold or release has
/// the user as actor; an automatic hold has the office. A failed record
/// write is the bridge's to report: the hold itself is already stored.
///
/// The release of an agent hold also clears that agent's Needs you item when
/// the item says the office held it (0008 D2). The office hold is not an
/// agent hold, so its release clears nothing.
pub struct HoldJournalBridge {
    record: RecordActionHandler,
    needs_you: NeedsYouBridge,
    hub: EventHub,
    alerts: StorageAlerts,
}

impl HoldJournalBridge {
    #[must_use]
    pub fn new(
        record: RecordActionHandler,
        needs_you: NeedsYouBridge,
        hub: EventHub,
        alerts: StorageAlerts,
    ) -> Self {
        Self {
            record,
            needs_you,
            hub,
            alerts,
        }
    }

    fn record_entry(&self, actor: Actor, scope: HoldScope, action: ActionKind, at: Timestamp) {
        let target_agent_id = match scope {
            HoldScope::Office => None,
            HoldScope::Agent(agent_id) => Some(agent_id),
        };
        let stored = self.record.handle(&ActionRecord {
            actor,
            target_agent_id,
            action,
            result: ActionResult::Ok,
            at,
        });
        match stored {
            Ok(entry) => self.hub.publish(&action_recorded_event(&entry)),
            Err(error) => self
                .alerts
                .report_disk_failure(&format!("the action record failed: {error}")),
        }
    }
}

impl HoldJournal for HoldJournalBridge {
    fn held(&self, hold: &Hold) {
        let actor = match hold.set_by {
            HoldSetter::User => Actor::User,
            HoldSetter::Office => Actor::Office,
        };
        self.record_entry(actor, hold.scope, ActionKind::Hold, hold.since);
    }

    fn released(&self, scope: HoldScope) {
        self.record_entry(Actor::User, scope, ActionKind::Release, SystemClock.now());
        if let HoldScope::Agent(agent_id) = scope {
            self.needs_you
                .clear(agent_id, NeedsYouClearReason::HoldReleased);
        }
    }

    fn storage_failed(&self, message: &str) {
        self.alerts.report_disk_failure(&format!(
            "the automatic hold could not be stored: {message}"
        ));
    }
}

/// Tells the user about an automatic hold: one `auto_hold` notification
/// naming the limit, and `hold.changed`. This is the only publish of an
/// automatic hold, whichever request caused it.
pub struct HoldNotifierBridge {
    notifications: NotificationBridge,
    store: Arc<dyn HoldStore>,
    hub: EventHub,
    alerts: StorageAlerts,
}

impl HoldNotifierBridge {
    #[must_use]
    pub fn new(
        notifications: NotificationBridge,
        store: Arc<dyn HoldStore>,
        hub: EventHub,
        alerts: StorageAlerts,
    ) -> Self {
        Self {
            notifications,
            store,
            hub,
            alerts,
        }
    }
}

impl HoldNotifier for HoldNotifierBridge {
    fn auto_held(&self, agent: AgentId, limit: &LimitRefusal) {
        self.notifications
            .record_auto_hold(agent, limit.kind.human_name());
        // The service stored the hold before it called, so the store holds
        // the state the clients must see.
        match self.store.load_holds() {
            Ok(holds) => self
                .hub
                .publish(&hold_changed_event(&HoldsView::from_holds(&holds))),
            Err(error) => self
                .alerts
                .report_disk_failure(&format!("the holds could not be read: {error}")),
        }
    }
}

#[cfg(test)]
#[path = "hold_bridges.tests.rs"]
mod tests;
