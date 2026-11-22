// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use hierarchy_domain::{HierarchyError, descendants_of};
use kernel::AgentId;

use crate::common::errors::HierarchyApplicationError;
use crate::common::interfaces::HierarchyStore;
use crate::common::ports::{
    SubtreeAgentEnder, SubtreeAgentRemover, SubtreeWorktrees, WorktreeReport,
};

/// What `agent.stop` and `agent.delete` carry when they reach a subtree
/// (FR-006, FR-007).
#[derive(Debug, Clone)]
pub struct SubtreeRequest {
    pub agent_id: AgentId,
    /// Stop or delete the descendants in the same action.
    pub include_descendants: bool,
    /// What the client showed in its confirmation. The server re-counts and
    /// refuses a stale number.
    pub confirmed_descendants: Option<usize>,
    /// What a delete does with the agents' own worktrees (0010).
    pub worktrees: SubtreeWorktrees,
}

/// The agents a subtree delete removed and what became of their worktrees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtreeDeletion {
    /// Deepest first, the requested agent last.
    pub deleted: Vec<AgentId>,
    pub worktrees: WorktreeReport,
}

/// Stops or deletes an agent and, when asked, its whole subtree, deepest
/// first (FR-006, FR-007). Only the user reaches this handler; agents never
/// delete (A4).
pub struct SubtreeHandler {
    store: Arc<dyn HierarchyStore>,
    ender: Arc<dyn SubtreeAgentEnder>,
    remover: Arc<dyn SubtreeAgentRemover>,
}

impl SubtreeHandler {
    #[must_use]
    pub fn new(
        store: Arc<dyn HierarchyStore>,
        ender: Arc<dyn SubtreeAgentEnder>,
        remover: Arc<dyn SubtreeAgentRemover>,
    ) -> Self {
        Self {
            store,
            ender,
            remover,
        }
    }

    /// The number of descendants, for the client's confirmation question.
    ///
    /// # Errors
    /// `Storage` when the read fails.
    pub fn descendant_count(&self, agent_id: AgentId) -> Result<usize, HierarchyApplicationError> {
        Ok(descendants_of(&self.store.snapshot()?.rows, agent_id).len())
    }

    /// Stops the agent and, when asked, its descendants: deepest first, the
    /// agent itself last (FR-007). The tree exclusion held across the count
    /// and the stops keeps the stopped set the one the count confirmed.
    ///
    /// # Errors
    /// `Invalid` with `DescendantCountChanged` when the tree changed since
    /// the client asked; the error of the first agent that cannot be stopped.
    pub fn stop_subtree(
        &self,
        request: &SubtreeRequest,
    ) -> Result<Vec<AgentId>, HierarchyApplicationError> {
        let _tree = self.store.tree_exclusion();
        let stopped = self.affected(request)?;
        for agent in &stopped {
            self.ender
                .stop(*agent)
                .map_err(HierarchyApplicationError::from_stop)?;
        }
        Ok(stopped)
    }

    /// Deletes the agent and, when asked, its whole subtree: deepest first,
    /// the agent itself last, so a failed step leaves no parent deleted
    /// above a child (FR-006). The tree exclusion held across the count and
    /// the deletes keeps the confirmed count and the deleted set one tree:
    /// no placement can join in between and be missed.
    ///
    /// Every agent's worktree confirmation is checked, every agent whose
    /// worktree goes is stopped, and every confirmation is checked again
    /// before the first delete, so one question covers the whole subtree
    /// and a refusal deletes nobody (0010 FR-005, D6).
    ///
    /// # Errors
    /// `Conflict` when the agent still has descendants and the request does
    /// not confirm their fresh count, or a worktree changed since the user
    /// confirmed it; the error of the first agent that cannot be deleted.
    /// The agents deleted before that step stay deleted.
    pub fn delete_subtree(
        &self,
        request: &SubtreeRequest,
    ) -> Result<SubtreeDeletion, HierarchyApplicationError> {
        let _tree = self.store.tree_exclusion();
        let deleted = self.affected_for_delete(request)?;
        for agent in &deleted {
            self.remover
                .check(*agent, &request.worktrees)
                .map_err(HierarchyApplicationError::from_delete)?;
        }
        for agent in &deleted {
            self.remover
                .prepare(*agent, &request.worktrees)
                .map_err(HierarchyApplicationError::from_delete)?;
        }
        let mut worktrees = WorktreeReport::default();
        for agent in &deleted {
            let report = self
                .remover
                .delete(*agent, &request.worktrees)
                .map_err(HierarchyApplicationError::from_delete)?;
            worktrees.extend(report);
        }
        Ok(SubtreeDeletion { deleted, worktrees })
    }

    /// The agents a stop affects: the agent plus its descendants when asked,
    /// after the count check.
    fn affected(
        &self,
        request: &SubtreeRequest,
    ) -> Result<Vec<AgentId>, HierarchyApplicationError> {
        if !request.include_descendants {
            return Ok(vec![request.agent_id]);
        }
        let descendants = self.descendants(request.agent_id)?;
        if let Some(confirmed) = request.confirmed_descendants
            && confirmed != descendants.len()
        {
            return Err(HierarchyApplicationError::Invalid(
                HierarchyError::DescendantCountChanged {
                    confirmed,
                    actual: descendants.len(),
                },
            ));
        }
        Ok(agent_last(descendants, request.agent_id))
    }

    /// The agents a delete affects. An agent without descendants still
    /// deletes alone, as in 0001; one with descendants deletes its subtree
    /// only when the request confirms the fresh count, so no descendant is
    /// left without its parent (FR-006).
    fn affected_for_delete(
        &self,
        request: &SubtreeRequest,
    ) -> Result<Vec<AgentId>, HierarchyApplicationError> {
        let descendants = self.descendants(request.agent_id)?;
        if descendants.is_empty() {
            return Ok(vec![request.agent_id]);
        }
        let count = descendants.len();
        let subject = if count == 1 {
            "descendant"
        } else {
            "descendants"
        };
        if !request.include_descendants {
            return Err(HierarchyApplicationError::Conflict(format!(
                "the agent still has {count} {subject}; delete them too by sending \
                 include_descendants=true and descendant_count {count}"
            )));
        }
        if request.confirmed_descendants != Some(count) {
            return Err(HierarchyApplicationError::Conflict(format!(
                "the agent has {count} {subject} now; send descendant_count {count} \
                 to confirm the delete"
            )));
        }
        Ok(agent_last(descendants, request.agent_id))
    }

    /// The agent's descendants, as storage holds the tree now.
    fn descendants(&self, agent_id: AgentId) -> Result<Vec<AgentId>, HierarchyApplicationError> {
        let snapshot = self.store.snapshot()?;
        Ok(descendants_of(&snapshot.rows, agent_id))
    }
}

/// Deepest first, the agent itself last: when a step fails mid-run, no
/// already-deleted agent sits above the failing one.
fn agent_last(mut descendants: Vec<AgentId>, agent_id: AgentId) -> Vec<AgentId> {
    descendants.push(agent_id);
    descendants
}

#[cfg(test)]
#[path = "subtree_handler.tests.rs"]
mod tests;
