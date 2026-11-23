// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::errors::AgentError;
use thiserror::Error;

use super::PortError;

/// The failure of a use case. Each variant maps to one wire error code
/// (`invalid`, `not_found`, `conflict`, `refused`, `internal`); the text
/// names the field or the entry at fault.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ApplicationError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Refused(String),
    #[error("{0}")]
    Internal(String),
    #[error("storage failure: {message}")]
    Storage {
        message: String,
        is_disk_failure: bool,
    },
}

impl From<AgentError> for ApplicationError {
    fn from(error: AgentError) -> Self {
        match error {
            // A shortcut that a fixed agent or switcher shortcut takes is a
            // conflict, not a malformed value (0003 contract).
            AgentError::ReservedShortcut { .. } => Self::Conflict(error.to_string()),
            // A full cap is the office's rule saying no, not a malformed
            // request (0020 D3).
            AgentError::TempCapReached { .. } => Self::Refused(error.to_string()),
            other => Self::Invalid(other.to_string()),
        }
    }
}

impl From<PortError> for ApplicationError {
    fn from(error: PortError) -> Self {
        match error {
            PortError::Duplicate { .. } | PortError::ProjectInUse { .. } => {
                Self::Conflict(error.to_string())
            }
            PortError::NotFound { .. } => Self::NotFound(error.to_string()),
            PortError::InvalidData { .. } => Self::Internal(error.to_string()),
            PortError::Storage {
                message,
                is_disk_failure,
            } => Self::Storage {
                message,
                is_disk_failure,
            },
        }
    }
}

impl ApplicationError {
    /// The same error with `detail` after its text: a failure that comes
    /// after a change the caller must still learn of.
    #[must_use]
    pub fn with_detail(self, detail: &str) -> Self {
        let add = |text: String| format!("{text}. {detail}");
        match self {
            Self::Invalid(text) => Self::Invalid(add(text)),
            Self::NotFound(text) => Self::NotFound(add(text)),
            Self::Conflict(text) => Self::Conflict(add(text)),
            Self::Refused(text) => Self::Refused(add(text)),
            Self::Internal(text) => Self::Internal(add(text)),
            Self::Storage {
                message,
                is_disk_failure,
            } => Self::Storage {
                message: add(message),
                is_disk_failure,
            },
        }
    }

    /// Like `From<PortError>`, but a duplicate names the field and the value.
    #[must_use]
    pub fn from_insert(error: PortError, field: &str, value: &str) -> Self {
        match error {
            PortError::Duplicate { .. } => {
                Self::Conflict(format!("{field} {value} is already used"))
            }
            other => other.into(),
        }
    }
}
