// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::value_objects::{EnvEntry, OfficeAdditions};

use crate::common::errors::ApplicationError;

/// Office additions as a client sent them, before the domain checks them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdditionsInput {
    pub flags: Vec<String>,
    pub env: Vec<(String, String)>,
    pub startup_prompt: Option<String>,
}

impl AdditionsInput {
    /// # Errors
    /// `Invalid`, naming the malformed environment entry or the empty flag.
    pub fn into_additions(self) -> Result<OfficeAdditions, ApplicationError> {
        let env = self
            .env
            .iter()
            .map(|(name, value)| EnvEntry::parse(name, value))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(OfficeAdditions::new(self.flags, env, self.startup_prompt)?)
    }
}
