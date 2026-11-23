// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_application::common::errors::PortError;
use agent_application::common::interfaces::{AgentStore, Clock};
use audit_application::commands::record_action::RecordActionHandler;
use audit_domain::{ActionKind, ActionRecord, ActionResult};
use audit_presentation::action_recorded_event;
use kernel::{Actor, AgentId, Timestamp};
use secret_application::{AgentDirectory, SecretClock, StoreError};
use secret_presentation::JournalEntry;

use super::SystemClock;
use crate::bootstrap::StorageAlerts;
use crate::server::EventHub;

/// Tells the secret service whether an agent exists, from the agent store.
pub struct AgentDirectoryBridge {
    agents: Arc<dyn AgentStore>,
}

impl AgentDirectoryBridge {
    #[must_use]
    pub fn new(agents: Arc<dyn AgentStore>) -> Self {
        Self { agents }
    }
}

impl AgentDirectory for AgentDirectoryBridge {
    fn exists(&self, agent: &AgentId) -> Result<bool, StoreError> {
        self.agents
            .find(*agent)
            .map(|found| found.is_some())
            .map_err(|error: PortError| StoreError {
                message: error.to_string(),
                is_disk_failure: false,
            })
    }
}

impl SecretClock for SystemClock {
    fn now(&self) -> Timestamp {
        Clock::now(self)
    }
}

/// Writes the reads of agents and the changes of the person into the action
/// record (0026/FR-010) and tells the clients with `action.recorded`. No text
/// holds a value. A failed write is the bridge's to report: the read or the
/// change already happened.
#[derive(Clone)]
pub struct SecretJournal {
    record: RecordActionHandler,
    hub: EventHub,
    alerts: StorageAlerts,
}

impl SecretJournal {
    #[must_use]
    pub fn new(record: RecordActionHandler, hub: EventHub, alerts: StorageAlerts) -> Self {
        Self {
            record,
            hub,
            alerts,
        }
    }

    /// Records what a request said to record, with the right actor.
    pub fn write(&self, entry: &JournalEntry) {
        match entry {
            JournalEntry::Read {
                agent,
                text,
                failed,
            } => self.append(
                Actor::Agent(*agent),
                Some(*agent),
                ActionKind::SecretRead,
                text,
                *failed,
            ),
            JournalEntry::Change { target, text } => {
                self.append(Actor::User, *target, ActionKind::SecretChange, text, false);
            }
        }
    }

    /// Records a read that a hold or a limit refused before the service ran.
    pub fn read_refused(&self, agent: AgentId, text: &str) {
        self.append(
            Actor::Agent(agent),
            Some(agent),
            ActionKind::SecretRead,
            text,
            true,
        );
    }

    fn append(
        &self,
        actor: Actor,
        target: Option<AgentId>,
        kind: ActionKind,
        text: &str,
        failed: bool,
    ) {
        let (result, detail) = if failed {
            (ActionResult::Failed(text.to_owned()), None)
        } else {
            (ActionResult::Ok, Some(text.to_owned()))
        };
        let stored = self.record.handle(&ActionRecord {
            actor,
            target_agent_id: target,
            action: kind,
            result,
            detail,
            at: Clock::now(&SystemClock),
            actor_label: None,
        });
        match stored {
            Ok(entry) => self.hub.publish(&action_recorded_event(&entry)),
            Err(error) => self
                .alerts
                .report_disk_failure(&format!("the action record failed: {error}")),
        }
    }
}
