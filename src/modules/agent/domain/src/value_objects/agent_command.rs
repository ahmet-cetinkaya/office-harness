// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use crate::errors::AgentError;

/// The CLI program and its fixed arguments, as the user typed them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCommand(String);

impl AgentCommand {
    /// # Errors
    /// `Empty` when the command has no text.
    pub fn parse(text: &str) -> Result<Self, AgentError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(AgentError::Empty { field: "command" });
        }
        Ok(Self(trimmed.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AgentCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
