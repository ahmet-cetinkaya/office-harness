// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use hierarchy_domain::entities::TeamId;
use hierarchy_domain::{HierarchyError, Placement, Role, check_placement, resolved_parent};
use kernel::AgentId;

use crate::common::errors::HierarchyApplicationError;
use crate::common::interfaces::HierarchyStore;

/// What `agent.create`'s `role` object carries (FR-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaceAgentRequest {
    pub agent_id: AgentId,
    pub role: Role,
    pub parent_agent_id: Option<AgentId>,
    pub team_id: Option<TeamId>,
}

/// Places an agent in the tree at a place the rules allow, or names the
/// roles that are allowed there (FR-003).
pub struct PlaceAgentHandler {
    store: Arc<dyn HierarchyStore>,
}

impl PlaceAgentHandler {
    #[must_use]
    pub fn new(store: Arc<dyn HierarchyStore>) -> Self {
        Self { store }
    }

    /// # Errors
    /// `Invalid` naming the broken rule: a second CEO, a parent that is not
    /// in the tree, or a role that cannot sit under that parent.
    pub fn handle(&self, request: PlaceAgentRequest) -> Result<(), HierarchyApplicationError> {
        // The exclusion covers the check and the write, so a placement that
        // races a subtree count never joins the tree unseen (FR-006).
        let _tree = self.store.tree_exclusion();
        let snapshot = self.store.snapshot()?;
        // A member's team is its lead's team (A6); the lead's team wins over
        // whatever the caller sent.
        let team_id = match request.role {
            Role::Member => member_team(&snapshot, request)?,
            other => request.team_id.filter(|_| other == Role::Lead),
        };
        let placement = Placement {
            agent_id: request.agent_id,
            role: request.role,
            parent_agent_id: request.parent_agent_id,
            team_id,
        };
        check_placement(&snapshot.rows, placement).map_err(HierarchyApplicationError::Invalid)?;
        self.store.place_agent(
            request.agent_id,
            Some(request.role),
            resolved_parent(&snapshot.rows, placement),
            team_id,
        )
    }
}

/// The team a member joins: its lead's team. A member without a lead, or
/// under a lead with no team, has no place.
fn member_team(
    snapshot: &crate::common::interfaces::TreeSnapshot,
    request: PlaceAgentRequest,
) -> Result<Option<TeamId>, HierarchyError> {
    let parent = request.parent_agent_id.ok_or(HierarchyError::LeadMissing)?;
    snapshot
        .rows
        .iter()
        .find(|row| row.agent_id == parent)
        .map(|row| row.team_id)
        .ok_or(HierarchyError::ParentNotFound)
}
