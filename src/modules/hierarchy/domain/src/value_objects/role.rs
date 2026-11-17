// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

use crate::errors::HierarchyError;

/// The role of an agent in the company tree: CEO, team lead, or member
/// (FR-001). An agent without a role is outside the hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Ceo,
    Lead,
    Member,
}

impl Role {
    /// The wire name of the role.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ceo => "ceo",
            Self::Lead => "lead",
            Self::Member => "member",
        }
    }

    /// The roles that may sit directly under an agent with this role. A CEO
    /// has team leads under it, a lead has members, and a member has no
    /// child agents in the office (FR-001, A4).
    #[must_use]
    pub fn allowed_child_roles(self) -> &'static [Role] {
        match self {
            Self::Ceo => &[Role::Lead],
            Self::Lead => &[Role::Member],
            Self::Member => &[],
        }
    }

    /// Reads a role from its wire name. `None` for any other text, so an
    /// unknown role is a refusal, never a guess.
    #[must_use]
    pub fn from_wire(text: &str) -> Option<Self> {
        match text {
            "ceo" => Some(Self::Ceo),
            "lead" => Some(Self::Lead),
            "member" => Some(Self::Member),
            _ => None,
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Parses a role from free text, naming the field at fault on failure.
///
/// # Errors
/// `UnknownRole` when the text is not one of the three wire names.
pub fn parse_role(field: &'static str, text: &str) -> Result<Role, HierarchyError> {
    Role::from_wire(text).ok_or_else(|| HierarchyError::UnknownRole {
        field,
        text: text.to_owned(),
    })
}

#[cfg(test)]
#[path = "role.tests.rs"]
mod tests;
