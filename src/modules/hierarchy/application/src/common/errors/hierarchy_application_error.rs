// SPDX-License-Identifier: AGPL-3.0-or-later

use hierarchy_domain::HierarchyError;
use thiserror::Error;

/// What a hierarchy use case can fail with, mapped to the wire error codes.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum HierarchyApplicationError {
    #[error("invalid: {0}")]
    Invalid(#[from] HierarchyError),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("storage failed")]
    Storage,
}

impl HierarchyApplicationError {
    /// Maps a stop or delete failure of one subtree member.
    #[must_use]
    pub fn from_stop(error: crate::common::ports::SubtreeActionError) -> Self {
        error.into_application()
    }

    /// Maps a delete failure of one subtree member.
    #[must_use]
    pub fn from_delete(error: crate::common::ports::SubtreeActionError) -> Self {
        error.into_application()
    }
}
