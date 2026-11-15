// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use crate::errors::AgentError;

const MAX_CHARACTERS: usize = 64;

/// The display name of an agent or a project: 1 to 64 characters after
/// trimming. Names are unique in the office without regard to case.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DisplayName(String);

impl DisplayName {
    /// # Errors
    /// `Empty` or `TooLong`, naming `field`.
    pub fn parse(field: &'static str, text: &str) -> Result<Self, AgentError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(AgentError::Empty { field });
        }
        if trimmed.chars().count() > MAX_CHARACTERS {
            return Err(AgentError::TooLong {
                field,
                max: MAX_CHARACTERS,
            });
        }
        Ok(Self(trimmed.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The text that decides uniqueness: the name in lower case.
    #[must_use]
    pub fn uniqueness_key(&self) -> String {
        self.0.to_lowercase()
    }
}

impl fmt::Display for DisplayName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
#[path = "display_name.tests.rs"]
mod tests;
