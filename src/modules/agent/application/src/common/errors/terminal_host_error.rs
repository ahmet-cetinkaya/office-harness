// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TerminalHostError {
    #[error("the agent already has a running terminal")]
    AlreadyRunning,
    #[error("the agent has no running terminal")]
    NotRunning,
    /// The command was not found, cannot start, or the child cannot be
    /// stopped. The text is shown to the user.
    #[error("{0}")]
    Failed(String),
}
