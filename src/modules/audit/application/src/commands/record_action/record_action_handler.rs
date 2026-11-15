// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use audit_domain::{ActionEntry, ActionRecord};

use crate::common::errors::AuditError;
use crate::common::interfaces::ActionLog;

/// Adds one entry to the action record (FR-011).
#[derive(Clone)]
pub struct RecordActionHandler {
    log: Arc<dyn ActionLog>,
}

impl RecordActionHandler {
    #[must_use]
    pub fn new(log: Arc<dyn ActionLog>) -> Self {
        Self { log }
    }

    /// # Errors
    /// The log's error; no entry was added.
    pub fn handle(&self, record: &ActionRecord) -> Result<ActionEntry, AuditError> {
        self.log.append(record)
    }
}
