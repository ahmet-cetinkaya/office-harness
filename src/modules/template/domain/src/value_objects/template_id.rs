// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;
use std::str::FromStr;

use uuid::Uuid;

use crate::errors::TemplateError;

/// Identifier of a template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TemplateId(Uuid);

impl TemplateId {
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for TemplateId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for TemplateId {
    type Err = TemplateError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(text)
            .map(Self)
            .map_err(|_| TemplateError::InvalidId {
                text: text.to_owned(),
            })
    }
}
