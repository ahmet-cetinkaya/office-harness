// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::errors::AgentError;
use kernel::AgentIdError;
use rusqlite::ffi::{ErrorCode, SQLITE_CONSTRAINT_UNIQUE};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("{what} already exists")]
    Duplicate { what: String },
    #[error("{what} was not found")]
    NotFound { what: String },
    #[error("project {project} is still used by the agents {}", .agents.join(", "))]
    ProjectInUse {
        project: String,
        agents: Vec<String>,
    },
    #[error("invalid data: {0}")]
    Invalid(#[from] AgentError),
    #[error("invalid data: {0}")]
    InvalidAgentId(#[from] AgentIdError),
    #[error("invalid data: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("storage failure: {0}")]
    Storage(#[from] rusqlite::Error),
}

impl PersistenceError {
    /// True when the disk, not the request, is at fault: it is full, read-only,
    /// or failing. The server reports these as `storage.error` and keeps running.
    #[must_use]
    pub fn is_storage_failure(&self) -> bool {
        let Self::Storage(error) = self else {
            return false;
        };
        matches!(
            error.sqlite_error_code(),
            Some(
                ErrorCode::DiskFull
                    | ErrorCode::SystemIoFailure
                    | ErrorCode::ReadOnly
                    | ErrorCode::CannotOpen
            )
        )
    }

    /// Turns a failed unique-name insert into `Duplicate` naming `what`.
    pub(crate) fn from_insert(error: rusqlite::Error, what: impl Into<String>) -> Self {
        let is_unique_violation = matches!(
            &error,
            rusqlite::Error::SqliteFailure(failure, _)
                if failure.extended_code == SQLITE_CONSTRAINT_UNIQUE
        );
        if is_unique_violation {
            Self::Duplicate { what: what.into() }
        } else {
            Self::Storage(error)
        }
    }
}
