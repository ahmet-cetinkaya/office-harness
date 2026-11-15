// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::errors::AgentError;

/// Where an agent stands. A new agent that never ran is `Stopped`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Starting,
    Working,
    WaitingForInput,
    Exited,
    Failed,
    Stopped,
    Interrupted,
}

impl AgentStatus {
    pub const ALL: [Self; 7] = [
        Self::Starting,
        Self::Working,
        Self::WaitingForInput,
        Self::Exited,
        Self::Failed,
        Self::Stopped,
        Self::Interrupted,
    ];

    /// Statuses of an agent that has a live session.
    pub const RUNNING: [Self; 3] = [Self::Starting, Self::Working, Self::WaitingForInput];

    #[must_use]
    pub fn is_running(self) -> bool {
        Self::RUNNING.contains(&self)
    }

    /// The stored and wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Working => "working",
            Self::WaitingForInput => "waiting_for_input",
            Self::Exited => "exited",
            Self::Failed => "failed",
            Self::Stopped => "stopped",
            Self::Interrupted => "interrupted",
        }
    }
}

impl fmt::Display for AgentStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for AgentStatus {
    type Err = AgentError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == text)
            .ok_or_else(|| AgentError::UnknownStatus(text.to_owned()))
    }
}
