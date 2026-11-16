// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TerminalHostError {
    #[error("the agent already has a running terminal")]
    AlreadyRunning,
    #[error("the agent has no running terminal")]
    NotRunning,
    /// The request itself is unsafe or wrong, so nothing is started and the
    /// agent's status does not change.
    #[error("{0}")]
    Refused(String),
    /// The command was not found, cannot start, or the child cannot be
    /// stopped. The text is shown to the user.
    #[error("{0}")]
    Failed(String),
}
