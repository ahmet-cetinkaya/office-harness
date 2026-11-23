// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::time::Duration;

use agent_application::common::interfaces::Clock;
use agent_application::common::models::StoredProcess;
use audit_domain::{ActionEntry, ActionKind, ActionRecord};
use audit_presentation::action_recorded_event;
use kernel::{Actor, AgentId};
use terminal_application::common::interfaces::ProcessInspector;
use tokio::task::JoinSet;

use super::identity_sweep::sweep_identity;
use super::leftover_result::LeftoverResult;
use super::visible_form::{escape_unsafe, visible_command};
use crate::bootstrap::{ServerLog, StorageAlerts};
use crate::server::EventHub;

/// A stored process identity of a `lost` session, with the agent it belongs to.
#[derive(Clone, Debug)]
pub struct Leftover {
    pub stored: StoredProcess,
    pub agent_id: AgentId,
}

/// Writes the entry of a sweep result and marks the identity as ended in one
/// step (0017/FR-009).
pub trait SweepLedger: Send + Sync {
    /// # Errors
    /// The text of the storage failure; neither write happened.
    fn record(&self, identity_id: i64, record: &ActionRecord) -> Result<ActionEntry, String>;
}

/// Tells the user that the sweep could not end a process of an agent.
pub trait LeftoverNotifier: Send + Sync {
    fn leftover_kept(&self, agent_id: AgentId, reason: &str);
}

/// Runs the sweep over the stored identities, all at the same time (spec A5).
#[derive(Clone)]
pub struct LeftoverSweep {
    pub inspector: Arc<dyn ProcessInspector>,
    pub ledger: Arc<dyn SweepLedger>,
    pub notifier: Arc<dyn LeftoverNotifier>,
    pub clock: Arc<dyn Clock>,
    pub events: EventHub,
    pub alerts: StorageAlerts,
    pub log: ServerLog,
}

/// The first reason an agent gave the sweep for a `could not end`.
struct KeptAgent {
    agent_id: AgentId,
    reason: String,
}

impl LeftoverSweep {
    /// Sweeps every identity, then sends one `leftover_kept` notification to
    /// each agent that has a `could not end` result (FR-006).
    pub async fn run(&self, leftovers: Vec<Leftover>, grace: Duration) {
        let mut tasks = JoinSet::new();
        for leftover in leftovers {
            let sweep = self.clone();
            tasks.spawn_blocking(move || sweep.settle(&leftover, grace));
        }
        let mut kept: Vec<KeptAgent> = Vec::new();
        while let Some(joined) = tasks.join_next().await {
            match joined {
                Ok(Some(agent)) => kept.push(agent),
                Ok(None) => {}
                Err(error) => self.log.error(&format!("a sweep task failed: {error}")),
            }
        }
        self.notify_once_per_agent(&kept);
    }

    /// Sweeps one identity and records its result.
    fn settle(&self, leftover: &Leftover, grace: Duration) -> Option<KeptAgent> {
        let result = sweep_identity(self.inspector.as_ref(), &leftover.stored.process, grace);
        self.record(leftover, &result);
        match result {
            LeftoverResult::CouldNotEnd(reason) => Some(KeptAgent {
                agent_id: leftover.agent_id,
                reason,
            }),
            LeftoverResult::Ended | LeftoverResult::EndedByForce | LeftoverResult::Left(_) => None,
        }
    }

    fn record(&self, leftover: &Leftover, result: &LeftoverResult) {
        let record = ActionRecord {
            actor: Actor::Office,
            target_agent_id: Some(leftover.agent_id),
            action: ActionKind::EndLeftover,
            result: result.action_result(),
            detail: Some(entry_detail(&leftover.stored, result)),
            at: self.clock.now(),
        };
        match self.ledger.record(leftover.stored.id, &record) {
            Ok(entry) => self.events.publish(&action_recorded_event(&entry)),
            Err(message) => self.alerts.report_disk_failure(&format!(
                "the sweep result of identity {} was not stored: {message}",
                leftover.stored.id
            )),
        }
    }

    fn notify_once_per_agent(&self, kept: &[KeptAgent]) {
        let mut notified: Vec<AgentId> = Vec::new();
        for agent in kept {
            if !notified.contains(&agent.agent_id) {
                notified.push(agent.agent_id);
                self.notifier
                    .leftover_kept(agent.agent_id, &escape_unsafe(&agent.reason));
            }
        }
    }
}

/// The detail of the action record entry (D6): it starts with the identity
/// row id, which is the idempotency key of the entry, then the result, the
/// process id, the session, and the command line in visible form.
fn entry_detail(stored: &StoredProcess, result: &LeftoverResult) -> String {
    format!(
        "identity {}: {}: pid {}, session {}, {}",
        stored.id,
        escape_unsafe(&result.text()),
        stored.process.pid,
        stored.session_id,
        visible_command(
            &stored.process.command_display,
            &stored.process.command_hash
        ),
    )
}

#[cfg(test)]
#[path = "leftover_sweep.tests.rs"]
mod tests;
