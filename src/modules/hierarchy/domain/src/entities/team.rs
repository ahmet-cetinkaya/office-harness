// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::value_objects::ProjectId;
use kernel::AgentId;
use uuid::Uuid;

use crate::value_objects::TeamName;

/// A team: a name, an optional project, and exactly one lead (FR-002).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Team {
    id: TeamId,
    name: TeamName,
    project_id: Option<ProjectId>,
    lead_agent_id: AgentId,
}

/// Identifier of a team, shared with the other layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TeamId(Uuid);

impl TeamId {
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    #[must_use]
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Builds an id from its raw value. Tests use this for fixed ids.
    #[must_use]
    pub fn from_u128(value: u128) -> Self {
        Self(Uuid::from_u128(value))
    }

    /// Reads the id as stored text. `None` for any text that is not a UUID.
    #[must_use]
    pub fn from_uuid_text(text: &str) -> Option<Self> {
        Uuid::parse_str(text).ok().map(Self)
    }

    #[must_use]
    pub fn as_uuid_text(&self) -> String {
        self.0.to_string()
    }
}

impl Team {
    #[must_use]
    pub fn new(name: TeamName, project_id: Option<ProjectId>, lead_agent_id: AgentId) -> Self {
        Self {
            id: TeamId::generate(),
            name,
            project_id,
            lead_agent_id,
        }
    }

    /// Rebuilds a team as storage read it.
    #[must_use]
    pub fn stored(
        id: TeamId,
        name: TeamName,
        project_id: Option<ProjectId>,
        lead_agent_id: AgentId,
    ) -> Self {
        Self {
            id,
            name,
            project_id,
            lead_agent_id,
        }
    }

    #[must_use]
    pub fn id(&self) -> TeamId {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &TeamName {
        &self.name
    }

    #[must_use]
    pub fn project_id(&self) -> Option<ProjectId> {
        self.project_id
    }

    #[must_use]
    pub fn lead_agent_id(&self) -> AgentId {
        self.lead_agent_id
    }
}
