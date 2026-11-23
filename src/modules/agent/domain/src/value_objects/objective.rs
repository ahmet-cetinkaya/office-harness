// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::errors::AgentError;

/// The most characters an objective holds, counted as Unicode scalar values.
pub const OBJECTIVE_MAX_CHARS: usize = 8192;

const FIELD: &str = "objective";

/// The first task of a temp agent. It follows the text rules of a directive
/// (0002): the objective is typed into the temp's terminal, so a control
/// character in it could act on that terminal. The agent module may not depend
/// on the hierarchy module, so the rules are restated here and the tests hold
/// the same table of cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Objective(String);

impl Objective {
    /// # Errors
    /// `Empty` for blank text, `TooLong` past `OBJECTIVE_MAX_CHARS`
    /// characters, and `ContainsControlCharacter` for any C0, DEL, or C1
    /// character.
    pub fn new(text: &str) -> Result<Self, AgentError> {
        if text.trim().is_empty() {
            return Err(AgentError::Empty { field: FIELD });
        }
        if text.chars().count() > OBJECTIVE_MAX_CHARS {
            return Err(AgentError::TooLong {
                field: FIELD,
                max: OBJECTIVE_MAX_CHARS,
            });
        }
        if text.chars().any(char::is_control) {
            return Err(AgentError::ContainsControlCharacter {
                field: FIELD.to_owned(),
            });
        }
        Ok(Self(text.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
#[path = "objective.tests.rs"]
mod tests;
