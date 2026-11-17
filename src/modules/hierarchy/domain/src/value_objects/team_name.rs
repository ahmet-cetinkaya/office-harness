// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::errors::HierarchyError;

/// A team name, unique in the office ignoring case (FR-002).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamName {
    display: String,
    /// The lower-cased name, the key uniqueness ignores case on.
    key: String,
}

const MAX_TEAM_NAME: usize = 64;

impl TeamName {
    /// Parses a team name. Leading and trailing spaces are not part of the
    /// name.
    ///
    /// # Errors
    /// `Empty` or `TooLong` naming the field.
    pub fn parse(text: &str) -> Result<Self, HierarchyError> {
        let display = text.trim().to_owned();
        if display.is_empty() {
            return Err(HierarchyError::Empty { field: "team name" });
        }
        if display.chars().count() > MAX_TEAM_NAME {
            return Err(HierarchyError::TooLong {
                field: "team name",
                max: MAX_TEAM_NAME,
            });
        }
        let key = display.to_lowercase();
        Ok(Self { display, key })
    }

    #[must_use]
    pub fn display(&self) -> &str {
        &self.display
    }

    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }
}

#[cfg(test)]
#[path = "team_name.tests.rs"]
mod tests;
