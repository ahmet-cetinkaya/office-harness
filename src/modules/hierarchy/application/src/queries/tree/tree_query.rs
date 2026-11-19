// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::sync::Arc;

use hierarchy_domain::entities::TeamId;
use hierarchy_domain::{Role, Team, TreeRow};
use kernel::AgentId;

use crate::common::errors::HierarchyApplicationError;
use crate::common::interfaces::HierarchyStore;

/// One agent as the tree shows it (FR-004).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeAgent {
    pub agent_id: AgentId,
    pub role: Option<Role>,
    pub team_id: Option<TeamId>,
}

/// One team branch: the lead and its members, in creation order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeTeam {
    pub team: Team,
    pub lead: TreeAgent,
    pub members: Vec<TreeAgent>,
}

/// The whole company in one read (FR-004).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tree {
    pub ceo: Option<TreeAgent>,
    pub teams: Vec<TreeTeam>,
    pub unassigned: Vec<TreeAgent>,
}

/// Reads the tree for `hierarchy.tree`.
pub struct TreeQuery {
    store: Arc<dyn HierarchyStore>,
}

impl TreeQuery {
    #[must_use]
    pub fn new(store: Arc<dyn HierarchyStore>) -> Self {
        Self { store }
    }

    /// # Errors
    /// `Storage` when the read fails.
    pub fn fetch(&self) -> Result<Tree, HierarchyApplicationError> {
        let snapshot = self.store.snapshot()?;
        Ok(assemble(&snapshot.rows, &snapshot.teams))
    }

    /// The tree rows as they are right now, for the rules that walk a chain
    /// (the agent-call descendant check, FR-008). The tree query itself
    /// assembles; this is the raw walk's input.
    ///
    /// # Errors
    /// `Storage` when the read fails.
    pub fn store_snapshot(&self) -> Result<Vec<TreeRow>, HierarchyApplicationError> {
        Ok(self.store.snapshot()?.rows)
    }

    /// Every placed agent's role and team name as the rows hold them now,
    /// for the agent rows the server answers with. An agent outside the map
    /// is outside the hierarchy.
    ///
    /// # Errors
    /// `Storage` when the read fails.
    pub fn row_places(&self) -> Result<HashMap<AgentId, RowPlace>, HierarchyApplicationError> {
        let snapshot = self.store.snapshot()?;
        Ok(row_places(&snapshot.rows, &snapshot.teams))
    }
}

/// One agent's place for its row: the role, and the team's name. The name
/// is display text read at the time of the answer, never a key (joins go
/// through the tree), so a renamed team shows in the next answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowPlace {
    pub role: Option<Role>,
    pub team_name: Option<String>,
}

/// Maps each tree row to its place, naming its team from the team rows.
#[must_use]
pub fn row_places(rows: &[TreeRow], teams: &[Team]) -> HashMap<AgentId, RowPlace> {
    rows.iter()
        .map(|row| {
            let team_name = row.team_id.and_then(|team_id| {
                teams
                    .iter()
                    .find(|team| team.id() == team_id)
                    .map(|team| team.name().display().to_owned())
            });
            (
                row.agent_id,
                RowPlace {
                    role: row.role,
                    team_name,
                },
            )
        })
        .collect()
}

/// Assembles the tree: the CEO, every team with its lead and members, and
/// the agents that are outside the hierarchy.
#[must_use]
pub fn assemble(rows: &[TreeRow], teams: &[Team]) -> Tree {
    let mut tree = Tree::default();
    for row in rows {
        match row.role {
            Some(Role::Ceo) => tree.ceo = Some(TreeAgent::of(row)),
            Some(Role::Lead) => {
                // The row names the team it leads; matching the team by
                // its lead column instead would tie the tree to the seat's
                // stored state (a stale first lead_agent_id hides the team).
                if let Some(team) = teams.iter().find(|team| Some(team.id()) == row.team_id) {
                    tree.teams.push(TreeTeam {
                        team: team.clone(),
                        lead: TreeAgent::of(row),
                        members: rows
                            .iter()
                            .filter(|member| {
                                member.parent_agent_id == Some(row.agent_id)
                                    && member.role == Some(Role::Member)
                            })
                            .map(TreeAgent::of)
                            .collect(),
                    });
                }
            }
            Some(Role::Member) => {}
            None => tree.unassigned.push(TreeAgent::of(row)),
        }
    }
    tree
}

impl TreeAgent {
    fn of(row: &TreeRow) -> Self {
        Self {
            agent_id: row.agent_id,
            role: row.role,
            team_id: row.team_id,
        }
    }
}

#[cfg(test)]
#[path = "tree_query.tests.rs"]
mod tests;
