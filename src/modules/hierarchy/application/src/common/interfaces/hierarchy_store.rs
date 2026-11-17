// SPDX-License-Identifier: AGPL-3.0-or-later

use hierarchy_domain::entities::TeamId;
use hierarchy_domain::value_objects::{Role, TeamName};
use hierarchy_domain::{Team, TreeRow};
use kernel::AgentId;

use crate::common::errors::HierarchyApplicationError;

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

    /// Writes one agent's place columns (role, parent, team).
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
