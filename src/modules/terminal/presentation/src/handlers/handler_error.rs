// SPDX-License-Identifier: AGPL-3.0-or-later

use terminal_application::common::errors::TerminalRuntimeError;
use terminal_domain::errors::TerminalError;

use crate::handlers::ErrorKind;

/// A failed request: the code and a message that names the field or entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerError {
    pub kind: ErrorKind,
    pub message: String,
}

impl From<TerminalError> for HandlerError {
    fn from(error: TerminalError) -> Self {
        Self {
            kind: ErrorKind::Invalid,
            message: error.to_string(),
        }
    }
}

impl From<TerminalRuntimeError> for HandlerError {
    fn from(error: TerminalRuntimeError) -> Self {
        let kind = match &error {
            TerminalRuntimeError::Invalid(_) => ErrorKind::Invalid,
            TerminalRuntimeError::NotRunning(_) | TerminalRuntimeError::NoOutput(_) => {
                ErrorKind::NotFound
            }
            TerminalRuntimeError::AlreadyRunning(_) => ErrorKind::Conflict,
            TerminalRuntimeError::SpawnFailed(_)
            | TerminalRuntimeError::Io(_)
            | TerminalRuntimeError::StopFailed => ErrorKind::Internal,
        };
        Self {
            kind,
            message: error.to_string(),
        }
    }
}
