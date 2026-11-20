// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use hierarchy_domain::entities::{Team, TeamId};
use hierarchy_domain::{Placement, Role, check_placement, resolved_parent};
use kernel::AgentId;

use crate::common::errors::HierarchyApplicationError;
use crate::common::interfaces::{HierarchyStore, TreeExclusion};
use crate::common::ports::{AgentCreator, AgentSeed, CreatorError};

/// What `agent.create` with a `role` object carries (FR-003): the agent's
/// own fields as a seed, plus the place it asks for in the tree.
#[derive(Debug, Clone)]
pub struct CreateRoleAgentRequest {
    pub seed: AgentSeed,
    pub role: Role,
    pub parent_agent_id: Option<AgentId>,
    pub team_id: Option<TeamId>,
}

/// Creates an agent and gives it its place in the tree in one step
/// (`agent.create` with `role`, FR-003). An agent the tree cannot place
/// leaves nothing behind.
pub struct CreateRoleAgentHandler {
    store: Arc<dyn HierarchyStore>,
    creator: Arc<dyn AgentCreator>,
}

impl CreateRoleAgentHandler {
    #[must_use]
    pub fn new(store: Arc<dyn HierarchyStore>, creator: Arc<dyn AgentCreator>) -> Self {
        Self { store, creator }
    }

    /// # Errors
    /// `Invalid` naming the broken place rule: a second CEO, a parent that
    /// is not in the tree, or a role that cannot sit under that parent.
    /// `Conflict` when the team asked for already holds its lead. The
    /// `InvalidInput`, `NotFoundInput`, `ConflictInput`, or `Storage` of the
    /// agent module's own refusal or failure, keeping its message.
    ///
    /// The caller holds the tree exclusion across this call, so a concurrent
    /// placement cannot join the tree between the check and the create+place
    /// (the subtree handlers' hold pattern, FR-006, FR-007): two callers
    /// racing for the last CEO seat see one tree.
    pub fn handle(
        &self,
        request: CreateRoleAgentRequest,
        // The caller's held exclusion; the parameter keeps the guard alive
        // for the whole body without a second, never-returning take.
        _tree: &TreeExclusion<'_>,
    ) -> Result<AgentId, HierarchyApplicationError> {
        let snapshot = self.store.snapshot()?;
        // A member's team is its lead's team (A6); the lead's team wins over
        // whatever the caller sent. This is the check half of the place use
        // case, before anything is created.
        let team_id = match request.role {
            Role::Member => member_team(&snapshot.rows, request.parent_agent_id)?,
            other => request.team_id.filter(|_| other == Role::Lead),
        };
        // The place is refused before anything is created, so a refused
        // create leaves no agent behind. The agent does not exist yet and
        // cannot collide with a row, so the check's id is a stand-in.
        let placement = Placement {
            agent_id: AgentId::generate(),
            role: request.role,
            parent_agent_id: request.parent_agent_id,
            team_id,
        };
        check_placement(&snapshot.rows, placement)
            .map_err(HierarchyApplicationError::from_place)?;
        let seed = with_team_project(request.seed, &snapshot.teams, team_id);
        let agent_id = self
            .creator
            .create(seed)
            .map_err(CreatorError::into_application)?;
        match self.store.place_agent(
            agent_id,
            Some(request.role),
            resolved_parent(
                &snapshot.rows,
                Placement {
                    agent_id,
                    ..placement
                },
            ),
            team_id,
        ) {
            Ok(()) => Ok(agent_id),
            // The placement failure is the one to report; the removal is
            // only the cleanup. When the cleanup also fails, the agent
            // stays behind unplaced, so its failure rides along in the
            // message and the user can find and delete the leftover.
            Err(placing) => Err(self.remove_or_absorb(agent_id, placing)),
        }
    }

    /// Removes the created agent again; when that cleanup fails too, the
    /// placement failure gains the cleanup's text instead of losing it.
    fn remove_or_absorb(
        &self,
        agent_id: AgentId,
        placing: HierarchyApplicationError,
    ) -> HierarchyApplicationError {
        let Err(removal) = self.creator.remove(agent_id) else {
            return placing;
        };
        if let HierarchyApplicationError::Storage(reason) = &placing {
            // Place failures are `Storage` by the store contract, so the
            // cleanup text rides in the same payload.
            return HierarchyApplicationError::Storage(format!(
                "{reason}; the unplaced agent {agent_id} could not be removed either: {}",
                removal.message
            ));
        }
        placing
    }
}

/// The seed with its team's project, when the team has one: a member's or
/// lead's project is its team's project, and an empty working folder then
/// means that project's folder (A6, 0001 A6). Without this, a member
/// created with no folder was refused for having neither.
fn with_team_project(seed: AgentSeed, teams: &[Team], team_id: Option<TeamId>) -> AgentSeed {
    let team_project = team_id
        .and_then(|id| teams.iter().find(|team| team.id() == id))
        .and_then(Team::project_id);
    match team_project {
        Some(project_id) => AgentSeed {
            project_id: Some(project_id),
            ..seed
        },
        None => seed,
    }
}

/// The team a member joins: its lead's team. A member without a lead, or
/// under a lead with no team, has no place.
fn member_team(
    rows: &[hierarchy_domain::TreeRow],
    parent_agent_id: Option<AgentId>,
) -> Result<Option<TeamId>, hierarchy_domain::HierarchyError> {
    let parent = parent_agent_id.ok_or(hierarchy_domain::HierarchyError::LeadMissing)?;
    rows.iter()
        .find(|row| row.agent_id == parent)
        .map(|row| row.team_id)
        .ok_or(hierarchy_domain::HierarchyError::ParentNotFound)
}

#[cfg(test)]
#[path = "create_role_agent_handler.tests.rs"]
mod tests;
