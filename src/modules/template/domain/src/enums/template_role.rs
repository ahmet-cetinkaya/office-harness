// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::errors::TemplateError;

/// The role an agent template asks for on a direct creation (A4). The text
/// forms are the ones of the 0002 wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateRole {
    Ceo,
    Lead,
    Member,
}

impl TemplateRole {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ceo => "ceo",
            Self::Lead => "lead",
            Self::Member => "member",
        }
    }

    /// # Errors
    /// `UnknownRole` for any text but `ceo`, `lead`, and `member`.
    pub fn parse(text: &str) -> Result<Self, TemplateError> {
        match text {
            "ceo" => Ok(Self::Ceo),
            "lead" => Ok(Self::Lead),
            "member" => Ok(Self::Member),
            other => Err(TemplateError::UnknownRole {
                role: other.to_owned(),
            }),
        }
    }
}
