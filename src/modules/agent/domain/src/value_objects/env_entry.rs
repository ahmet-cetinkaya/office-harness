// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::errors::AgentError;

/// One environment variable that an agent's CLI gets on top of the login
/// environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvEntry {
    name: String,
    value: String,
}

impl EnvEntry {
    /// # Errors
    /// `InvalidEnvName` naming the entry when the name does not match
    /// `[A-Za-z_][A-Za-z0-9_]*`, and `ContainsNul` for a value with a NUL.
    pub fn parse(name: &str, value: &str) -> Result<Self, AgentError> {
        if !is_valid_name(name) {
            return Err(AgentError::InvalidEnvName {
                name: name.to_owned(),
            });
        }
        if value.contains('\0') {
            return Err(AgentError::ContainsNul {
                field: format!("the value of environment entry {name}"),
            });
        }
        Ok(Self {
            name: name.to_owned(),
            value: value.to_owned(),
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

fn is_valid_name(name: &str) -> bool {
    let mut characters = name.chars();
    let starts_well = characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_');
    starts_well && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

#[cfg(test)]
#[path = "env_entry.tests.rs"]
mod tests;
