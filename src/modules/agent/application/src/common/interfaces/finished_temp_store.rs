// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::entities::FinishedTemp;
use agent_domain::value_objects::FinishedTempId;

use crate::common::errors::PortError;

/// The entries of temps that ended, kept until the person dismisses them
/// (0020 D11).
pub trait FinishedTempStore: Send + Sync {
    /// # Errors
    /// `Storage`.
    fn insert(&self, entry: &FinishedTemp) -> Result<(), PortError>;

    /// All entries, newest end first.
    ///
    /// # Errors
    /// `Storage`.
    fn list(&self) -> Result<Vec<FinishedTemp>, PortError>;

    /// Removes the entry and nothing else. `false` when no entry has the id.
    ///
    /// # Errors
    /// `Storage`.
    fn dismiss(&self, id: FinishedTempId) -> Result<bool, PortError>;
}
