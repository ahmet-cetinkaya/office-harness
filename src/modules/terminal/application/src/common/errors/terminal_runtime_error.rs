// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;
use terminal_domain::errors::TerminalError;
use thiserror::Error;

/// Why the terminal runtime refused or failed an operation.
#[derive(Debug, Error)]
pub enum TerminalRuntimeError {
    #[error(transparent)]
    Invalid(#[from] TerminalError),
    #[error("agent {0} has no running terminal")]
    NotRunning(AgentId),
    #[error("agent {0} already has a running terminal")]
    AlreadyRunning(AgentId),
    #[error("agent {0} has no running terminal and no saved output")]
    NoOutput(AgentId),
    #[error("the command could not start: {0}")]
    SpawnFailed(String),
    #[error("the terminal could not be written: {0}")]
    Io(String),
    #[error("the terminal did not end after the stop request")]
    StopFailed,
}
