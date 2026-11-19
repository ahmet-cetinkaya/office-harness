// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Mutex, MutexGuard, PoisonError};

use hierarchy_domain::entities::TeamId;
use hierarchy_domain::value_objects::{Role, TeamName};
use hierarchy_domain::{Team, TreeRow};
use kernel::AgentId;

use crate::common::errors::HierarchyApplicationError;

/// Keeps every tree writer off while a caller counts the tree and then
/// stops or deletes from that count, so the count and the writes see one
/// tree (FR-006, FR-007).
#[must_use]
pub struct TreeExclusion<'a> {
    /// The held writer lock; its `Drop` releases the tree to the next
    /// writer. Never read.
    _held: MutexGuard<'a, ()>,
}

impl<'a> TreeExclusion<'a> {
    /// Waits for the current holder, then takes the exclusion. A poisoned
    /// lock is reused, because a panic leaves the tree rows consistent.
    pub fn new(tree_writes: &'a Mutex<()>) -> Self {
        Self {
            _held: tree_writes.lock().unwrap_or_else(PoisonError::into_inner),
        }
    }
}

/// A team row as storage holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamRecord {
    pub team: Team,
}

/// Everything the tree rules and the tree query need, in one read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TreeSnapshot {
    pub rows: Vec<TreeRow>,
    pub teams: Vec<Team>,
}

/// Where the hierarchy use cases read and write their part of the office.
pub trait HierarchyStore: Send + Sync {
    /// The whole tree and every team, in one read for the tree query.
    ///
    /// # Errors
    /// `Storage` when the read fails.
    fn snapshot(&self) -> Result<TreeSnapshot, HierarchyApplicationError>;

    /// Keeps every tree writer of this store off until the returned guard
    /// drops. A caller that counts the tree and then stops or deletes from
    /// that count holds it across both, so a placement cannot join the tree
    /// in between (FR-006, FR-007).
    fn tree_exclusion(&self) -> TreeExclusion<'_>;

    /// Whether a team name is taken, ignoring case.
    ///
    /// # Errors
    /// `Storage` when the read fails.
    fn team_name_taken(&self, key: &str) -> Result<bool, HierarchyApplicationError>;

    /// Saves a new team and places its lead in one write. `parent_agent_id`
    /// is the lead's resolved parent (the CEO when one exists).
    ///
    /// # Errors
    /// `Storage` when the write fails; nothing is half-written.
    fn save_team_and_place_lead(
        &self,
        team: &Team,
        parent_agent_id: Option<AgentId>,
    ) -> Result<(), HierarchyApplicationError>;

    /// Removes a team record and unplaces its members in one write. The
    /// agents keep their rows; they leave the hierarchy.
    ///
    /// # Errors
    /// `Storage` when the write fails.
    fn delete_team_and_unplace(&self, team_id: TeamId) -> Result<(), HierarchyApplicationError>;

    /// Writes one agent's place columns (role, parent, team), and, for a
    /// lead with a team, names that agent the team's lead in the same
    /// write. The caller holds the tree exclusion across its check and this
    /// write; a second take inside would wait on itself (the create-role
    /// handler's hold pattern, FR-006, FR-007), so two racing lead
    /// placements cannot both win the seat.
    ///
    /// # Errors
    /// `Storage` when the write fails.
    fn place_agent(
        &self,
        agent_id: AgentId,
        role: Option<Role>,
        parent_agent_id: Option<AgentId>,
        team_id: Option<TeamId>,
    ) -> Result<(), HierarchyApplicationError>;

    /// The team names as the tree query shows them, in creation order.
    ///
    /// # Errors
    /// `Storage` when the read fails.
    fn team_name_of(&self, team_id: TeamId) -> Result<Option<TeamName>, HierarchyApplicationError>;
}
