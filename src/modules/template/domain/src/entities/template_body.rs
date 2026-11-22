// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::entities::{AgentBody, TeamBody};
use crate::enums::TemplateKind;
use crate::errors::TemplateError;

/// The definition a template holds: one agent or a team.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateBody {
    Agent(AgentBody),
    Team(TeamBody),
}

impl TemplateBody {
    #[must_use]
    pub const fn kind(&self) -> TemplateKind {
        match self {
            Self::Agent(_) => TemplateKind::Agent,
            Self::Team(_) => TemplateKind::Team,
        }
    }

    /// The save rules of FR-001, FR-003, and FR-010.
    ///
    /// # Errors
    /// The first rule that the body breaks.
    pub fn validate(&self) -> Result<(), TemplateError> {
        match self {
            Self::Agent(agent) => agent.validate("body"),
            Self::Team(team) => team.validate(),
        }
    }

    /// The body with every environment value dropped, for export and import
    /// (D3, A6).
    #[must_use]
    pub fn without_env_values(self) -> Self {
        match self {
            Self::Agent(agent) => Self::Agent(agent.without_env_values()),
            Self::Team(team) => Self::Team(team.without_env_values()),
        }
    }
}
