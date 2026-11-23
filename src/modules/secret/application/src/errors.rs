// SPDX-License-Identifier: AGPL-3.0-or-later

use secret_domain::{LockedReason, SecretError};
use thiserror::Error;

/// Why a secret request fails. No text holds a value or the store key.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SecretServiceError {
    #[error("{0}")]
    Invalid(#[from] SecretError),
    #[error("at most {max} secrets")]
    TooManySecrets { max: usize },
    #[error("no secret named {name}")]
    NoSuchSecret { name: String },
    #[error("agent {agent_id} does not exist")]
    NoSuchAgent { agent_id: String },
    #[error("the agent already holds this grant")]
    AlreadyGranted,
    #[error("the agent holds no grant")]
    NotGranted,
    /// One text for a missing, a deleted, and an ungranted secret (FR-007).
    #[error("no secret named {name} is available to this agent")]
    NotAvailable { name: String },
    #[error("the secret store is locked: {0}")]
    Locked(LockedReason),
    #[error("the secret {name} cannot be read")]
    CannotRead { name: String },
    #[error("the store holds {current} secrets, not the number confirmed")]
    ConfirmCountDiffers { current: usize },
    #[error("secret storage failed: {message}")]
    Storage {
        message: String,
        is_disk_failure: bool,
    },
}
