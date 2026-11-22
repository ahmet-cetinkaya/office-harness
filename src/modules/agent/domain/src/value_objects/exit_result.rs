// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;
use std::str::FromStr;

use crate::errors::AgentError;

const SIGNAL: &str = "signal";
const INTERRUPTED: &str = "interrupted";
const LOST: &str = "lost";
const SPAWN_ERROR_PREFIX: &str = "spawn-error: ";

/// How a session ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitResult {
    Code(u32),
    /// The user stopped the session.
    Signal,
    /// The CLI never started; the text says why.
    SpawnError(String),
    /// The server stopped while the session ran (FR-004).
    Interrupted,
    /// The session was still open when the server stopped without a clean
    /// stop; its CLI may still run (spec 0013 FR-008).
    Lost,
}

impl fmt::Display for ExitResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Code(code) => write!(formatter, "{code}"),
            Self::Signal => formatter.write_str(SIGNAL),
            Self::SpawnError(text) => write!(formatter, "{SPAWN_ERROR_PREFIX}{text}"),
            Self::Interrupted => formatter.write_str(INTERRUPTED),
            Self::Lost => formatter.write_str(LOST),
        }
    }
}

impl FromStr for ExitResult {
    type Err = AgentError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text == SIGNAL {
            return Ok(Self::Signal);
        }
        if text == INTERRUPTED {
            return Ok(Self::Interrupted);
        }
        if text == LOST {
            return Ok(Self::Lost);
        }
        if let Some(reason) = text.strip_prefix(SPAWN_ERROR_PREFIX) {
            return Ok(Self::SpawnError(reason.to_owned()));
        }
        text.parse()
            .map(Self::Code)
            .map_err(|_| AgentError::UnknownExitResult(text.to_owned()))
    }
}

#[cfg(test)]
#[path = "exit_result.tests.rs"]
mod tests;
