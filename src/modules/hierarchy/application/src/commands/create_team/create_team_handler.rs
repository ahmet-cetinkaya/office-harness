// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use hierarchy_domain::value_objects::TeamName;
use hierarchy_domain::{Placement, Role, Team, check_placement, resolved_parent};
use kernel::AgentId;
use kernel::ProjectId;

use crate::common::errors::HierarchyApplicationError;
use crate::common::interfaces::HierarchyStore;

/// What `team.create` carries.
#[derive(Debug, Clone)]
pub struct CreateTeamRequest {
    pub name: String,
    pub project_id: Option<ProjectId>,
    /// The lead agent, already created by the agent module with no role.
    pub lead_agent_id: AgentId,
}

/// Creates a team and places its lead under the CEO (FR-002). One write: the
/// team row and the lead's place columns go together or not at all.
pub struct CreateTeamHandler {
    store: Arc<dyn HierarchyStore>,
}

impl CreateTeamHandler {
    #[must_use]
    pub fn new(store: Arc<dyn HierarchyStore>) -> Self {
        Self { store }
    }

    /// # Errors
    /// `Invalid` for a bad name; `Conflict` when the name is taken; `Invalid`
    /// when the placement breaks a tree rule, naming the reason.
    pub fn handle(&self, request: &CreateTeamRequest) -> Result<Team, HierarchyApplicationError> {
        let name = TeamName::parse(&request.name)?;
        if self.store.team_name_taken(name.key())? {
            return Err(HierarchyApplicationError::Conflict(format!(
                "a team named {} already exists",
                name.display()
            )));
        }
        let team = Team::new(name, request.project_id, request.lead_agent_id);
        let snapshot = self.store.snapshot()?;
        // The lead is checked against the current tree first, so a refusal
        // writes nothing.
        let placement = Placement {
            agent_id: request.lead_agent_id,
            role: Role::Lead,
            parent_agent_id: None,
            team_id: Some(team.id()),
        };
        check_placement(&snapshot.rows, placement).map_err(HierarchyApplicationError::Invalid)?;
        let parent = resolved_parent(&snapshot.rows, placement);
        self.store.save_team_and_place_lead(&team, parent)?;
        Ok(team)
    }
}
