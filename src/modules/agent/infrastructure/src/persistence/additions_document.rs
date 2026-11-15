// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::value_objects::{EnvEntry, OfficeAdditions};
use serde::{Deserialize, Serialize};

use super::PersistenceError;

/// The stored form of office additions: the same JSON in the additions table
/// and in a session's snapshot.
#[derive(Serialize, Deserialize)]
pub struct AdditionsDocument {
    flags: Vec<String>,
    env: Vec<(String, String)>,
    startup_prompt: Option<String>,
}

impl AdditionsDocument {
    #[must_use]
    pub fn of(additions: &OfficeAdditions) -> Self {
        Self {
            flags: additions.flags().to_vec(),
            env: additions
                .env()
                .iter()
                .map(|entry| (entry.name().to_owned(), entry.value().to_owned()))
                .collect(),
            startup_prompt: additions.startup_prompt().map(str::to_owned),
        }
    }

    /// The stored text is checked again by the domain rules, so a damaged
    /// row cannot give an invalid addition to a start.
    ///
    /// # Errors
    /// `InvalidJson`, or `Invalid` when the domain refuses the stored values.
    pub fn parse(text: &str) -> Result<OfficeAdditions, PersistenceError> {
        let document: Self = serde_json::from_str(text)?;
        let env = document
            .env
            .iter()
            .map(|(name, value)| EnvEntry::parse(name, value))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(OfficeAdditions::new(
            document.flags,
            env,
            document.startup_prompt,
        )?)
    }

    /// # Errors
    /// `InvalidJson` when the document cannot be written as text.
    pub fn text_of(additions: &OfficeAdditions) -> Result<String, PersistenceError> {
        Ok(serde_json::to_string(&Self::of(additions))?)
    }
}

#[cfg(test)]
#[path = "additions_document.tests.rs"]
mod tests;
