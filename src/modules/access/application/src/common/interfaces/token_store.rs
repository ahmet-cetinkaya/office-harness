// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::common::errors::AccessError;
use crate::common::models::TokenSnapshot;

/// Keeps the office token, which survives a restart.
pub trait TokenStore: Send + Sync {
    /// The token now in force.
    ///
    /// # Errors
    /// `Storage` or `InvalidToken` when the token cannot be read.
    fn current(&self) -> Result<TokenSnapshot, AccessError>;

    /// Replaces the token with a new random one and raises the generation.
    ///
    /// # Errors
    /// `Storage` when the new token cannot be saved, `Randomness` when the
    /// system has no random bytes. The old token stays in force.
    fn regenerate(&self) -> Result<TokenSnapshot, AccessError>;
}
