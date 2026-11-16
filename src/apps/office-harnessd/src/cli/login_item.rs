// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;

use thiserror::Error;

/// A failure of the operating system's login item facility.
#[derive(Debug, Error)]
#[error("login item: {0}")]
pub struct LoginItemError(pub String);

/// The operating system's way to start a program when the user logs in. A
/// trait, so that tests never touch the real user's login items.
pub trait LoginItem {
    /// Starts `program` at each login. Enabling again replaces the entry.
    ///
    /// # Errors
    /// The operating system refused the entry.
    fn enable(&self, program: &Path) -> Result<(), LoginItemError>;

    /// Removes the entry. Succeeds when there is none.
    ///
    /// # Errors
    /// The operating system refused to remove the entry.
    fn disable(&self) -> Result<(), LoginItemError>;

    /// # Errors
    /// The operating system did not answer.
    fn is_enabled(&self) -> Result<bool, LoginItemError>;
}
