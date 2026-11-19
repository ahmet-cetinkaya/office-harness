// SPDX-License-Identifier: AGPL-3.0-or-later

use hierarchy_domain::HierarchyError;
use thiserror::Error;

/// What a hierarchy use case can fail with, mapped to the wire error codes.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum HierarchyApplicationError {
    #[error("invalid: {0}")]
    Invalid(#[from] HierarchyError),
    /// The agent module refused what it was given on its own rules; the
    /// text is its message alone and the wire code stays `invalid`, so a
    /// seeded create answers with exactly the text `agent.create` answers
    /// with (0001, and #33 r2 LOW: no `invalid: ` prefix in front of a
    /// message that already speaks for itself).
    #[error("{0}")]
    InvalidInput(String),
    /// The agent module could not find what the create named; the text is
    /// its message alone, so the role route answers with exactly what a
    /// plain `agent.create` answers (Efe, PR 40 LOW: no `not found: `
    /// prefix in front of a pass-through message).
    #[error("{0}")]
    NotFoundInput(String),
    /// The agent module refused the create as a conflict (a taken name or
    /// resource); the text is its message alone, like `InvalidInput`.
    #[error("{0}")]
    ConflictInput(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    /// A hold or a rate limit refused the action; the text is the reason
    /// alone, and the wire code is `refused` (0007).
    #[error("{0}")]
    Refused(String),
    #[error("storage failed: {0}")]
    Storage(String),
}

impl HierarchyApplicationError {
    /// Maps a broken place rule to its wire class: a second lead is a
    /// conflict, because the seat is taken, and keeps the domain rule's own
    /// wording; every other broken rule stays invalid (FR-003).
    #[must_use]
    pub fn from_place(error: HierarchyError) -> Self {
        match error {
            HierarchyError::SecondLead { .. } | HierarchyError::LeadsAnotherTeam => {
                Self::Conflict(error.to_string())
            }
            other => Self::Invalid(other),
        }
    }

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
