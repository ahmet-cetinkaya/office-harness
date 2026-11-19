// SPDX-License-Identifier: AGPL-3.0-or-later

use audit_domain::{ActionEntry, ActionRecord};
use kernel::AgentId;

use crate::common::errors::AuditError;

/// Where the action record lives.
pub trait ActionLog: Send + Sync {
    /// Stores the record and returns the entry with its id.
    ///
    /// # Errors
    /// `Storage` when the record cannot be written.
    fn append(&self, record: &ActionRecord) -> Result<ActionEntry, AuditError>;

    /// The entries of one target agent, oldest first.
    ///
    /// # Errors
    /// `Storage` or `Corrupt`.
    fn list_for_agent(&self, agent_id: AgentId) -> Result<Vec<ActionEntry>, AuditError>;

    /// The entries without a target agent, newest first, at most 500.
    ///
    /// # Errors
    /// `Storage` or `Corrupt`.
    fn list_office_level(&self) -> Result<Vec<ActionEntry>, AuditError>;
}
