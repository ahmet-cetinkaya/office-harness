// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::common::errors::PortError;
use crate::common::models::AgentActionEntry;

/// Where the use cases write the action record. The host app implements it
/// with the audit module.
pub trait ActionRecorder: Send + Sync {
    /// # Errors
    /// `Storage` when the entry cannot be written.
    fn record(&self, entry: &AgentActionEntry) -> Result<(), PortError>;
}
