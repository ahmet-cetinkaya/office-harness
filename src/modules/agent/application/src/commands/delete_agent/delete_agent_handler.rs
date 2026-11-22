// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use agent_domain::entities::Agent;
use agent_domain::value_objects::AgentWorktree;
use kernel::AgentId;

use crate::common::action_journal::ActionJournal;
use crate::common::errors::{ApplicationError, TerminalHostError};
use crate::common::interfaces::{
    AgentStore, SettingsStore, TerminalHost, WorktreeManager, WorktreeStatus,
};
use crate::common::lookup::require_agent;
use crate::common::models::AgentAction;

/// What a delete does with the own worktrees of the agents it deletes
/// (0010 FR-005).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum DeleteWorktrees {
    /// The folder and the branch stay, and the answer names them (A6).
    #[default]
    Keep,
    /// Removes each worktree whose status still has the hash the user
    /// confirmed, keyed by agent.
    Remove(HashMap<AgentId, String>),
    /// The undo of a create: removes the worktree and its branch without
    /// asking. Never on the wire; only `AgentCreator::remove` uses it.
    Discard,
}

/// The branches and folders a delete removed or kept (FR-007).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeOutcome {
    pub deleted_branches: Vec<String>,
    pub kept_branches: Vec<String>,
    pub kept_folders: Vec<String>,
}

impl WorktreeOutcome {
    /// Adds another agent's outcome, for a subtree delete.
    pub fn extend(&mut self, other: Self) {
        self.deleted_branches.extend(other.deleted_branches);
        self.kept_branches.extend(other.kept_branches);
        self.kept_folders.extend(other.kept_folders);
    }

    /// The outcome as one sentence, or `None` when it names nothing.
    #[must_use]
    pub fn describe(&self) -> Option<String> {
        let parts: Vec<String> = [
            ("deleted branches", &self.deleted_branches),
            ("kept branches", &self.kept_branches),
            ("kept folders", &self.kept_folders),
        ]
        .into_iter()
        .filter(|(_, names)| !names.is_empty())
        .map(|(label, names)| format!("{label}: {}", names.join(", ")))
        .collect();
        (!parts.is_empty()).then(|| parts.join("; "))
    }

    fn kept(worktree: &AgentWorktree) -> Self {
        Self {
            kept_branches: vec![worktree.branch.clone()],
            kept_folders: vec![worktree.folder.to_string_lossy().into_owned()],
            ..Self::default()
        }
    }
}

/// Deletes an agent (`agent.delete`, FR-012). A running agent is stopped
/// first. The action record keeps the entries of the deleted agent. An own
/// worktree is kept or removed as the request says (0010 D6).
#[derive(Clone)]
pub struct DeleteAgentHandler {
    agents: Arc<dyn AgentStore>,
    settings: Arc<dyn SettingsStore>,
    terminals: Arc<dyn TerminalHost>,
    journal: ActionJournal,
    /// Absent keeps every worktree: nothing could read or remove one.
    worktrees: Option<Arc<dyn WorktreeManager>>,
}

impl DeleteAgentHandler {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        settings: Arc<dyn SettingsStore>,
        terminals: Arc<dyn TerminalHost>,
        journal: ActionJournal,
    ) -> Self {
        Self {
            agents,
            settings,
            terminals,
            journal,
            worktrees: None,
        }
    }

    /// Attaches the git worktree service (0010).
    #[must_use]
    pub fn with_worktrees(mut self, worktrees: Arc<dyn WorktreeManager>) -> Self {
        self.worktrees = Some(worktrees);
        self
    }

    /// Checks, without changing anything, that a remove still matches what
    /// the user confirmed, so a subtree delete can refuse before its first
    /// step.
    ///
    /// # Errors
    /// `NotFound`; `Conflict` naming the agent whose worktree changed.
    pub fn check(
        &self,
        agent_id: AgentId,
        choice: &DeleteWorktrees,
    ) -> Result<(), ApplicationError> {
        let agent = require_agent(&*self.agents, agent_id)?;
        let (Some(worktree), DeleteWorktrees::Remove(confirmed)) = (&agent.worktree, choice) else {
            return Ok(());
        };
        let status = self.status(worktree);
        require_confirmed(&agent, &status, confirmed)
    }

    /// Stops an agent whose worktree the delete removes, then checks its
    /// status again: the last step of a subtree's preparation, so no member
    /// writes after its final check.
    ///
    /// # Errors
    /// `NotFound`; `Internal` when the child cannot be ended; `Conflict`
    /// when the worktree changed, which leaves the agent stopped.
    pub fn stop_and_check(
        &self,
        agent_id: AgentId,
        choice: &DeleteWorktrees,
    ) -> Result<(), ApplicationError> {
        let agent = require_agent(&*self.agents, agent_id)?;
        let (Some(worktree), DeleteWorktrees::Remove(confirmed)) = (&agent.worktree, choice) else {
            return Ok(());
        };
        self.stop_if_running(&agent)?;
        require_confirmed_after_stop(&agent, &self.status(worktree), confirmed)
    }

    /// The agent's row goes before git touches its worktree, so a failed
    /// row delete leaves the worktree as it was (D6).
    ///
    /// # Errors
    /// `NotFound`; `Internal` when a running child cannot be ended, which
    /// keeps the agent; `Conflict` when a worktree to remove changed since
    /// the user confirmed it, which keeps the agent (stopped) and its
    /// worktree. A failure to record the delete comes after the worktree
    /// step, so its text names what that step removed and kept.
    pub fn handle(
        &self,
        agent_id: AgentId,
        choice: &DeleteWorktrees,
    ) -> Result<WorktreeOutcome, ApplicationError> {
        let agent = require_agent(&*self.agents, agent_id)?;
        let step = self.prepare(&agent, choice)?;
        self.agents.delete(agent_id)?;
        let outcome = self.finish(step);
        self.journal
            .record(agent_id, AgentAction::Delete, None)
            .map_err(|error| match outcome.describe() {
                Some(text) => error.with_detail(&format!("The agent was deleted; {text}")),
                None => error,
            })?;
        Ok(outcome)
    }

    /// Everything before the row delete: the D6 check, the stop, and the
    /// check again, since the agent may have written a file up to its stop.
    fn prepare<'a>(
        &self,
        agent: &'a Agent,
        choice: &DeleteWorktrees,
    ) -> Result<WorktreeStep<'a>, ApplicationError> {
        let Some(worktree) = &agent.worktree else {
            self.stop_if_running(agent)?;
            return Ok(WorktreeStep::Nothing);
        };
        let DeleteWorktrees::Remove(confirmed) = choice else {
            self.stop_if_running(agent)?;
            return Ok(if *choice == DeleteWorktrees::Discard {
                WorktreeStep::Discard(worktree)
            } else {
                WorktreeStep::Keep(worktree)
            });
        };
        require_confirmed(agent, &self.status(worktree), confirmed)?;
        self.stop_if_running(agent)?;
        let status = self.status(worktree);
        require_confirmed_after_stop(agent, &status, confirmed)?;
        Ok(WorktreeStep::Remove(worktree, status))
    }

    fn finish(&self, step: WorktreeStep<'_>) -> WorktreeOutcome {
        match step {
            WorktreeStep::Nothing => WorktreeOutcome::default(),
            WorktreeStep::Keep(worktree) => WorktreeOutcome::kept(worktree),
            WorktreeStep::Discard(worktree) => {
                self.discard(worktree);
                WorktreeOutcome::default()
            }
            WorktreeStep::Remove(worktree, status) => self.remove_worktree(worktree, &status),
        }
    }

    /// The last D6 steps: remove, then the branch. A worktree that cannot be
    /// read or removed is kept and named.
    fn remove_worktree(
        &self,
        worktree: &AgentWorktree,
        status: &WorktreeStatus,
    ) -> WorktreeOutcome {
        let Some(worktrees) = self.worktrees.as_ref().filter(|_| status.reachable) else {
            return WorktreeOutcome::kept(worktree);
        };
        // A worktree confirmed clean goes without force, so git itself
        // refuses one that changed in the last moment.
        let force = status.changed_files > 0 || !status.folder_exists;
        if worktrees.remove(worktree, force).is_err() {
            return WorktreeOutcome::kept(worktree);
        }
        let branch_deleted =
            status.branch_is_deletable() && worktrees.delete_branch(worktree).is_ok();
        if branch_deleted {
            WorktreeOutcome {
                deleted_branches: vec![worktree.branch.clone()],
                ..WorktreeOutcome::default()
            }
        } else {
            WorktreeOutcome {
                kept_branches: vec![worktree.branch.clone()],
                ..WorktreeOutcome::default()
            }
        }
    }

    fn status(&self, worktree: &AgentWorktree) -> WorktreeStatus {
        self.worktrees.as_ref().map_or_else(
            || WorktreeStatus::unreachable(worktree),
            |worktrees| worktrees.status(worktree),
        )
    }

    fn discard(&self, worktree: &AgentWorktree) {
        if let Some(worktrees) = &self.worktrees {
            worktrees.discard(worktree);
        }
    }

    fn stop_if_running(&self, agent: &Agent) -> Result<(), ApplicationError> {
        if !agent.status.is_running() {
            return Ok(());
        }
        let grace = Duration::from_millis(self.settings.load()?.stop_grace_ms);
        match self.terminals.stop(agent.id, grace) {
            // The child ended on its own a moment ago.
            Ok(()) | Err(TerminalHostError::NotRunning) => Ok(()),
            Err(error) => Err(ApplicationError::Internal(error.to_string())),
        }
    }
}

/// What the delete does to the agent's worktree once its row is gone.
enum WorktreeStep<'a> {
    Nothing,
    Keep(&'a AgentWorktree),
    Discard(&'a AgentWorktree),
    Remove(&'a AgentWorktree, WorktreeStatus),
}

/// A reachable worktree must still have the status the user confirmed; one
/// that cannot be read is kept anyway, so it needs no confirmation.
fn require_confirmed(
    agent: &Agent,
    status: &WorktreeStatus,
    confirmed: &HashMap<AgentId, String>,
) -> Result<(), ApplicationError> {
    if !status.reachable || confirmed.get(&agent.id) == Some(&status.status_hash) {
        return Ok(());
    }
    Err(ApplicationError::Conflict(format!(
        "the worktree of {} changed; confirm again",
        agent.name
    )))
}

/// As `require_confirmed`, after the stop: the refusal says that the agent
/// stays stopped.
fn require_confirmed_after_stop(
    agent: &Agent,
    status: &WorktreeStatus,
    confirmed: &HashMap<AgentId, String>,
) -> Result<(), ApplicationError> {
    require_confirmed(agent, status, confirmed).map_err(|_| {
        ApplicationError::Conflict(format!(
            "the worktree of {} changed while it stopped; it is stopped now, confirm again",
            agent.name
        ))
    })
}

#[cfg(test)]
#[path = "delete_agent_handler.tests.rs"]
mod tests;
