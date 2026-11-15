// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

/// A failure of a store or of the action recorder, as the host app reports it.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PortError {
    #[error("{what} already exists")]
    Duplicate { what: String },
    #[error("{what} was not found")]
    NotFound { what: String },
    #[error("project {project} is still used by the agents {}", .agents.join(", "))]
    ProjectInUse {
        project: String,
        agents: Vec<String>,
    },
    /// A stored row that the domain rules refuse, so the store is damaged.
    #[error("stored data is invalid: {message}")]
    InvalidData { message: String },
    #[error("storage failure: {message}")]
    Storage {
        message: String,
        /// True when the disk is at fault, which the server reports as
        /// `storage.error` and survives.
        is_disk_failure: bool,
    },
}
