// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Identifier of an agent, shared by every module that refers to one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentId(Uuid);

#[derive(Debug, Error, PartialEq, Eq)]
#[error("agent id is not a valid UUID: {0}")]
pub struct AgentIdError(String);

impl AgentId {
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for AgentId {
    type Err = AgentIdError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(text)
            .map(Self)
            .map_err(|_| AgentIdError(text.to_owned()))
    }
}

#[cfg(test)]
#[path = "agent_id.tests.rs"]
mod tests;
