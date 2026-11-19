// SPDX-License-Identifier: AGPL-3.0-or-later

use hierarchy_domain::{Hold, HoldScope};
use kernel::AgentId;

use crate::common::errors::HierarchyApplicationError;

/// Where the holds live between runs. One row per scope.
pub trait HoldStore: Send + Sync {
    /// # Errors
    /// `Storage` when the read fails.
    fn load_holds(&self) -> Result<Vec<Hold>, HierarchyApplicationError>;

    /// Saves one hold, replacing a stored hold of the same scope.
    ///
    /// # Errors
    /// `Storage` when the write fails.
    fn save_hold(&self, hold: &Hold) -> Result<(), HierarchyApplicationError>;

    /// # Errors
    /// `Storage` when the write fails.
    fn delete_hold(&self, scope: HoldScope) -> Result<(), HierarchyApplicationError>;

    /// Removes the holds of agents that are gone, in one write.
    ///
    /// # Errors
    /// `Storage` when the write fails; nothing is half-deleted.
    fn delete_agent_holds(&self, agent_ids: &[AgentId]) -> Result<(), HierarchyApplicationError>;
}
