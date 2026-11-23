// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, PoisonError};

use agent_infrastructure::persistence::SessionProcessRepository;
use audit_domain::{ActionEntry, ActionRecord};
use audit_infrastructure::persistence::ActionRepository;
use rusqlite::Connection;

use super::leftover_sweep::SweepLedger;

/// Writes the sweep result of one identity: the action record entry and the
/// `ended` mark of the identity, in one transaction (0017/FR-009). The agent
/// and audit tables live in the one office database and share its connection,
/// so one transaction can hold both writes.
pub struct SharedLedger {
    connection: Arc<Mutex<Connection>>,
}

impl SharedLedger {
    #[must_use]
    pub fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }
}

impl SweepLedger for SharedLedger {
    fn record(&self, identity_id: i64, record: &ActionRecord) -> Result<ActionEntry, String> {
        // A panic in another call leaves SQLite consistent, so a poisoned
        // lock is safe to reuse.
        let mut connection = self
            .connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        let entry =
            ActionRepository::append_in(&transaction, record).map_err(|error| error.to_string())?;
        SessionProcessRepository::end_one_in(&transaction, identity_id)
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(entry)
    }
}

#[cfg(test)]
#[path = "leftover_ledger.tests.rs"]
mod tests;
