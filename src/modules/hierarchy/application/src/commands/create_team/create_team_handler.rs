// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use hierarchy_domain::value_objects::TeamName;
use hierarchy_domain::{Placement, Role, Team, check_placement, resolved_parent};
use kernel::AgentId;
use kernel::ProjectId;

use crate::common::errors::HierarchyApplicationError;
use crate::common::interfaces::HierarchyStore;
use crate::common::ports::{AgentCreator, AgentSeed, CreatorError};

/// What `team.create` carries.
#[derive(Debug, Clone)]
pub struct CreateTeamRequest {
    pub name: String,
    pub project_id: Option<ProjectId>,
    /// The lead agent, already created by the agent module with no role.
    pub lead_agent_id: AgentId,
}

/// What `team.create` carries when the lead is a new agent: the contract's
/// `lead` object, created by the agent module as `agent.create` does.
#[derive(Debug, Clone)]
pub struct CreateTeamWithLeadRequest {
    pub name: String,
    pub project_id: Option<ProjectId>,
    pub lead: AgentSeed,
}

/// Creates a team and places its lead under the CEO (FR-002). One write: the
/// team row and the lead's place columns go together or not at all.
pub struct CreateTeamHandler {
    store: Arc<dyn HierarchyStore>,
    creator: Option<Arc<dyn AgentCreator>>,
}

impl CreateTeamHandler {
    #[must_use]
    pub fn new(store: Arc<dyn HierarchyStore>) -> Self {
        Self {
            store,
            creator: None,
        }
    }

    /// The agent module's create, which a team with a new lead needs.
    #[must_use]
    pub fn with_creator(mut self, creator: Arc<dyn AgentCreator>) -> Self {
        self.creator = Some(creator);
        self
    }

    /// # Errors
    /// `Invalid` for a bad name; `Conflict` when the name is taken or the
    /// lead already leads another team; `Invalid` when the placement breaks
    /// another tree rule, naming the reason.
    pub fn handle(&self, request: &CreateTeamRequest) -> Result<Team, HierarchyApplicationError> {
        let name = self.free_name(&request.name)?;
        let team = Team::new(name, request.project_id, request.lead_agent_id);
        let snapshot = self.store.snapshot()?;
        // The lead is checked against the current tree first, so a refusal
        // writes nothing.
        let placement = lead_placement(&team);
        check_placement(&snapshot.rows, placement)
            .map_err(HierarchyApplicationError::from_place)?;
        let parent = resolved_parent(&snapshot.rows, placement);
        self.store.save_team_and_place_lead(&team, parent)?;
        Ok(team)
    }

    /// Creates the lead agent, then the team as `handle` does (the
    /// contract's `lead` object). The name and the lead's place are checked
    /// before the agent exists, so a refused team creates no agent; a team
    /// write that fails afterwards removes the new lead again.
    ///
    /// # Errors
    /// As `handle`, plus the agent module's own refusal of the lead, and
    /// `Storage` when no agent creator is attached.
    pub fn handle_with_new_lead(
        &self,
        request: CreateTeamWithLeadRequest,
    ) -> Result<Team, HierarchyApplicationError> {
        let creator = self.creator.as_ref().ok_or_else(|| {
            HierarchyApplicationError::Storage("no agent creator is attached".to_owned())
        })?;
        let name = self.free_name(&request.name)?;
        // The lead does not exist yet, so the check uses a stand-in id.
        let stand_in = Team::new(name, request.project_id, AgentId::generate());
        check_placement(&self.store.snapshot()?.rows, lead_placement(&stand_in))
            .map_err(HierarchyApplicationError::from_place)?;
        let lead_agent_id = creator
            .create(request.lead)
            .map_err(CreatorError::into_application)?;
        self.handle(&CreateTeamRequest {
            name: request.name,
            project_id: request.project_id,
            lead_agent_id,
        })
        .map_err(|failure| remove_lead(creator.as_ref(), lead_agent_id, failure))
    }

    fn free_name(&self, text: &str) -> Result<TeamName, HierarchyApplicationError> {
        let name = TeamName::parse(text)?;
        if self.store.team_name_taken(name.key())? {
            return Err(HierarchyApplicationError::Conflict(format!(
                "a team named {} already exists",
                name.display()
            )));
        }
        Ok(name)
    }
}

/// The place a team's lead takes: under the CEO, leading this team.
fn lead_placement(team: &Team) -> Placement {
    Placement {
        agent_id: team.lead_agent_id(),
        role: Role::Lead,
        parent_agent_id: None,
        team_id: Some(team.id()),
    }
}

/// Removes the new lead after the team failed. The team failure is the one
/// to report; when the removal fails too, the leftover agent is named so
/// the user can find and delete it.
fn remove_lead(
    creator: &dyn AgentCreator,
    lead_agent_id: AgentId,
    failure: HierarchyApplicationError,
) -> HierarchyApplicationError {
    match creator.remove(lead_agent_id) {
        Ok(()) => failure,
        Err(removal) => HierarchyApplicationError::Storage(format!(
            "{failure}; the new lead {lead_agent_id} could not be removed either: {}",
            removal.message
        )),
    }
}

#[cfg(test)]
#[path = "create_team_handler.tests.rs"]
mod tests;
