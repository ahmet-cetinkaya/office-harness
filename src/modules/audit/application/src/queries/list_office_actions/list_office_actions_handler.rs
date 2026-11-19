// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use audit_domain::ActionEntry;

use crate::common::errors::AuditError;
use crate::common::interfaces::ActionLog;

/// The office-level entries, newest first (`office.actions`).
#[derive(Clone)]
pub struct ListOfficeActionsHandler {
    log: Arc<dyn ActionLog>,
}

impl ListOfficeActionsHandler {
    #[must_use]
    pub fn new(log: Arc<dyn ActionLog>) -> Self {
        Self { log }
    }

    /// # Errors
    /// The log's error.
    pub fn handle(&self) -> Result<Vec<ActionEntry>, AuditError> {
        self.log.list_office_level()
    }
}
