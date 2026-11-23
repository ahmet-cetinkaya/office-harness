// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::value_objects::SessionId;

use crate::common::errors::PortError;
use crate::common::models::{ProcessKind, SessionProcess, StoredProcess};

/// The stored identities of the processes of running sessions. A session
/// that ends marks its identities as ended in the same step as the session
/// row, so this port has no call for that.
pub trait SessionProcessStore: Send + Sync {
    /// Stores the process as open. Returns false, and stores nothing, when
    /// the session already holds a row for this process id and start time or
    /// is no longer running.
    ///
    /// # Errors
    /// `Storage`.
    fn insert_open(
        &self,
        session_id: SessionId,
        kind: ProcessKind,
        process: &SessionProcess,
    ) -> Result<bool, PortError>;

    /// # Errors
    /// `Storage`.
    fn open_for_session(&self, session_id: SessionId) -> Result<Vec<StoredProcess>, PortError>;

    /// Replaces the command line of a row that replaced itself with another
    /// program; the row stays open.
    ///
    /// # Errors
    /// `Storage`.
    fn update_command(&self, id: i64, process: &SessionProcess) -> Result<(), PortError>;

    /// Deletes a helper row. A CLI row is never deleted this way.
    ///
    /// # Errors
    /// `Storage`.
    fn drop_helper(&self, id: i64) -> Result<(), PortError>;

    /// Marks the session as having left helpers unstored (the helper limit).
    ///
    /// # Errors
    /// `Storage`.
    fn mark_helper_limit(&self, session_id: SessionId) -> Result<(), PortError>;
}
