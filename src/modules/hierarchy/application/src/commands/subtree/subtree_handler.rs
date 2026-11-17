// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use hierarchy_domain::{HierarchyError, descendants_of};
use kernel::AgentId;

use crate::common::errors::HierarchyApplicationError;
use crate::common::interfaces::HierarchyStore;
use crate::common::ports::{SubtreeAgentEnder, SubtreeAgentRemover};

/// What `agent.stop` and `agent.delete` carry when they reach a subtree
/// (FR-006, FR-007).
#[derive(Debug, Clone, Copy)]
pub struct SubtreeRequest {
    pub agent_id: AgentId,
    /// Stop or delete the descendants in the same action.
    pub include_descendants: bool,
    /// What the client showed in its confirmation. The server re-counts and
    /// refuses a stale number.
    pub confirmed_descendants: Option<usize>,
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

    /// Stops the agent and, when asked, its descendants.
    ///
    /// # Errors
    /// `Invalid` with `DescendantCountChanged` when the tree changed since
    /// the client asked; the error of the first agent that cannot be stopped.
    pub fn stop_subtree(
        &self,
        request: SubtreeRequest,
    ) -> Result<Vec<AgentId>, HierarchyApplicationError> {
        let stopped = self.affected(request)?;
        for agent in &stopped {
            self.ender
                .stop(*agent)
                .map_err(HierarchyApplicationError::from_stop)?;
        }
        Ok(stopped)
    }

    /// Deletes the agent and, when asked, its descendants, deepest first.
    ///
    /// # Errors
    /// `Invalid` with `DescendantCountChanged` when the tree changed since
    /// the client asked; the error of the first agent that cannot be deleted.
    pub fn delete_subtree(
        &self,
        request: SubtreeRequest,
    ) -> Result<Vec<AgentId>, HierarchyApplicationError> {
        let deleted = self.affected(request)?;
        // Deepest first, so no parent leaves a child behind mid-run.
        for agent in &deleted {
            self.remover
                .delete(*agent)
                .map_err(HierarchyApplicationError::from_delete)?;
        }
        Ok(deleted)
    }

    /// The agents an action affects: the agent plus its descendants when
    /// asked, after the count check.
    fn affected(&self, request: SubtreeRequest) -> Result<Vec<AgentId>, HierarchyApplicationError> {
        if !request.include_descendants {
            return Ok(vec![request.agent_id]);
        }
        let snapshot = self.store.snapshot()?;
        let mut affected = descendants_of(&snapshot.rows, request.agent_id);
        if let Some(confirmed) = request.confirmed_descendants
            && confirmed != affected.len()
        {
            return Err(HierarchyApplicationError::Invalid(
                HierarchyError::DescendantCountChanged {
                    confirmed,
                    actual: affected.len(),
                },
            ));
        }
        affected.insert(0, request.agent_id);
        Ok(affected)
    }
}
