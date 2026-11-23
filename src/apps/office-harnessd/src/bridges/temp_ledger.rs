// SPDX-License-Identifier: AGPL-3.0-or-later

//! What the office keeps of a temp after it ended (0020 D11): the finished
//! entry, the `end` and `dismiss` action record entries, and the
//! `temp.changed` event.

use std::path::PathBuf;
use std::sync::Arc;

use agent_application::common::action_journal::ActionJournal;
use agent_application::common::actor_scope::ActorScope;
use agent_application::common::errors::PortError;
use agent_application::common::interfaces::{Clock, FinishedTempStore};
use agent_application::common::models::AgentAction;
use agent_domain::entities::{Agent, FinishedReport, FinishedTemp};
use agent_domain::value_objects::{AgentWorktree, FinishedTempId};
use kernel::{Actor, AgentId, Timestamp};
use serde_json::Value;
use wire::Event;

use crate::bootstrap::StorageAlerts;
use crate::server::{EventHub, recorded_and_published};

/// The event that tells clients to read `temp.list` again.
pub const TEMP_CHANGED_EVENT: &str = "temp.changed";

const WORKTREE_STAYS: &str = "the worktree could not be removed";
const BRANCH_STAYS: &str = "the branch could not be removed";

#[must_use]
pub fn temp_changed_event() -> Event {
    Event::new(TEMP_CHANGED_EVENT, serde_json::Map::new())
}

/// What a temp was, read before its agent is deleted: the agent is gone when
/// the entry is written.
#[derive(Debug, Clone)]
pub struct TempSnapshot {
    pub agent_id: AgentId,
    pub name: String,
    pub objective: String,
    pub creator: Actor,
    pub parent_name: Option<String>,
    pub started_at: Timestamp,
    pub worktree: Option<AgentWorktree>,
}

impl TempSnapshot {
    /// `None` for an agent that is not a temp.
    #[must_use]
    pub fn of(agent: &Agent, parent_name: Option<String>) -> Option<Self> {
        let temp = agent.temp.as_ref()?;
        Some(Self {
            agent_id: agent.id,
            name: agent.name.as_str().to_owned(),
            objective: temp.objective.as_str().to_owned(),
            creator: temp.creator,
            parent_name,
            started_at: agent.created_at,
            worktree: agent.worktree.clone(),
        })
    }
}

/// How and why a temp ended.
#[derive(Debug, Clone)]
pub struct EndNote {
    pub reason: String,
    /// The person, the stopping agent, or the office.
    pub actor: Actor,
    /// The `done` report, when the end followed one.
    pub report: Option<FinishedReport>,
}

/// The worktree folder and branch that stayed, and why one could not go.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeptWork {
    pub folder: Option<PathBuf>,
    pub branch: Option<String>,
    pub removal_failure: Option<String>,
}

impl KeptWork {
    /// Reads what stayed from the answer of `agent.delete`. `removal_asked`
    /// says the delete tried to remove the worktree, so a folder or branch
    /// that stayed is a failure and not the choice to keep it.
    #[must_use]
    pub fn from_outcome(snapshot: &TempSnapshot, outcome: &Value, removal_asked: bool) -> Self {
        let Some(worktree) = &snapshot.worktree else {
            return Self::default();
        };
        let names = |key: &str| -> Vec<&str> {
            outcome
                .get(key)
                .and_then(Value::as_array)
                .map_or_default(|names| names.iter().filter_map(Value::as_str).collect())
        };
        let folder_stays =
            names("kept_folders").contains(&worktree.folder.to_string_lossy().as_ref());
        let branch_stays = names("kept_branches").contains(&worktree.branch.as_str());
        let removal_failure = match (removal_asked, folder_stays, branch_stays) {
            (true, true, _) => Some(WORKTREE_STAYS.to_owned()),
            (true, false, true) => Some(BRANCH_STAYS.to_owned()),
            _ => None,
        };
        Self {
            folder: folder_stays.then(|| worktree.folder.clone()),
            branch: branch_stays.then(|| worktree.branch.clone()),
            removal_failure,
        }
    }
}

/// Writes the finished entries and the `end` records.
#[derive(Clone)]
pub struct TempLedger {
    finished: Arc<dyn FinishedTempStore>,
    journal: ActionJournal,
    clock: Arc<dyn Clock>,
    events: EventHub,
    alerts: StorageAlerts,
}

impl TempLedger {
    #[must_use]
    pub fn new(
        finished: Arc<dyn FinishedTempStore>,
        journal: ActionJournal,
        clock: Arc<dyn Clock>,
        events: EventHub,
        alerts: StorageAlerts,
    ) -> Self {
        Self {
            finished,
            journal,
            clock,
            events,
            alerts,
        }
    }

    /// Stores the entry, records the end under its actor, and tells the
    /// clients. A failed write is raised as a storage alert and loses only
    /// that write.
    pub fn record(&self, snapshot: &TempSnapshot, end: &EndNote, kept: KeptWork) {
        let entry = FinishedTemp {
            id: FinishedTempId::generate(),
            agent_id: snapshot.agent_id,
            name: snapshot.name.clone(),
            objective: snapshot.objective.clone(),
            creator: snapshot.creator,
            parent_name: snapshot.parent_name.clone(),
            started_at: snapshot.started_at,
            ended_at: self.clock.now(),
            reason: end.reason.clone(),
            report: end.report.clone(),
            kept_folder: kept.folder,
            kept_branch: kept.branch,
            removal_failure: kept.removal_failure,
        };
        if let Err(error) = self.finished.insert(&entry) {
            self.alerts
                .report_disk_failure(&format!("the finished temp entry failed: {error}"));
        }
        self.record_end(snapshot.agent_id, end);
        self.events.publish(&temp_changed_event());
    }

    /// Removes a finished entry and records the dismiss on the ended temp's
    /// record. `Ok(false)` when no entry has the id.
    ///
    /// # Errors
    ///
    /// The store could not read or remove the entry.
    pub fn dismiss(&self, entry_id: FinishedTempId) -> Result<bool, PortError> {
        let entry = self
            .finished
            .list()?
            .into_iter()
            .find(|entry| entry.id == entry_id);
        let Some(entry) = entry else {
            return Ok(false);
        };
        if !self.finished.dismiss(entry_id)? {
            return Ok(false);
        }
        let entry_text = entry_id.to_string();
        let recorded = recorded_and_published(&self.events, || {
            self.journal.record_detailed(
                entry.agent_id,
                AgentAction::Dismiss,
                None,
                Some(&entry_text),
            )
        });
        if let Err(error) = recorded {
            self.alerts
                .report_disk_failure(&format!("the action record failed: {error}"));
        }
        self.events.publish(&temp_changed_event());
        Ok(true)
    }

    fn record_end(&self, agent_id: AgentId, end: &EndNote) {
        let _actor = ActorScope::enter(end.actor);
        let recorded = recorded_and_published(&self.events, || {
            self.journal
                .record_detailed(agent_id, AgentAction::End, None, Some(&end.reason))
        });
        if let Err(error) = recorded {
            self.alerts
                .report_disk_failure(&format!("the action record failed: {error}"));
        }
    }
}

#[cfg(test)]
#[path = "temp_ledger.tests.rs"]
mod tests;
