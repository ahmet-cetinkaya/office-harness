// SPDX-License-Identifier: AGPL-3.0-or-later

use access_domain::errors::TokenError;
use thiserror::Error;

/// No variant carries the token. The texts name the failure only.
#[derive(Debug, Error)]
pub enum AccessError {
    #[error("the proof is not valid")]
    ProofRejected,
    #[error("the proof cannot be computed")]
    ProofUnavailable,
    #[error(transparent)]
    InvalidToken(#[from] TokenError),
    #[error("the token file failed: {0}")]
    Storage(String),
    #[error("the system gave no random bytes: {0}")]
    Randomness(String),
}
