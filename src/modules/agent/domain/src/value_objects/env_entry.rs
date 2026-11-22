// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::errors::AgentError;

/// One environment variable that an agent's CLI gets on top of the login
/// environment. A variable without a value ("no value", `None`) is never
/// passed to the CLI, so the login environment's value stays in effect; an
/// empty text (`Some("")`) is a value set on purpose (0009/FR-011).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvEntry {
    name: String,
    value: Option<String>,
}

/// Names with this prefix belong to the server: it sets them for the agent's
/// call credential (0002) and removes any other. An office addition cannot
/// set one.
pub const RESERVED_ENV_PREFIX: &str = "OFFICE_HARNESS_";

impl EnvEntry {
    /// # Errors
    /// `InvalidEnvName` naming the entry when the name does not match
    /// `[A-Za-z_][A-Za-z0-9_]*`, and `ContainsNul` for a value with a NUL.
    pub fn parse(name: &str, value: Option<&str>) -> Result<Self, AgentError> {
        if !is_valid_name(name) {
            return Err(AgentError::InvalidEnvName {
                name: name.to_owned(),
            });
        }
        if name.starts_with(RESERVED_ENV_PREFIX) {
            return Err(AgentError::ReservedEnvName {
                name: name.to_owned(),
            });
        }
        if value.is_some_and(|text| text.contains('\0')) {
            return Err(AgentError::ContainsNul {
                field: format!("the value of environment entry {name}"),
            });
        }
        Ok(Self {
            name: name.to_owned(),
            value: value.map(str::to_owned),
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
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
