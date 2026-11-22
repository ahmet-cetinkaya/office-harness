// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::errors::TemplateError;

/// Whether a template defines one agent or a team (D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateKind {
    Agent,
    Team,
}

impl TemplateKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Team => "team",
        }
    }

    /// # Errors
    /// `UnknownKind` for any text but `agent` and `team`.
    pub fn parse(text: &str) -> Result<Self, TemplateError> {
        match text {
            "agent" => Ok(Self::Agent),
            "team" => Ok(Self::Team),
            other => Err(TemplateError::UnknownKind {
                kind: other.to_owned(),
            }),
        }
    }
}
