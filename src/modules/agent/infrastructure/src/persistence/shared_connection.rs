// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, PoisonError};

use rusqlite::Connection;

use super::PersistenceError;

/// One SQLite connection that the repositories of the module share.
#[derive(Clone)]
pub struct SharedConnection(Arc<Mutex<Connection>>);

impl SharedConnection {
    #[must_use]
    pub fn new(connection: Connection) -> Self {
        Self(Arc::new(Mutex::new(connection)))
    }

    /// The connection itself, for the other modules that the host app gives
    /// the same database file (each keeps its own tables).
    #[must_use]
    pub fn handle(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.0)
    }

    pub(crate) fn with<T>(
        &self,
        work: impl FnOnce(&mut Connection) -> Result<T, PersistenceError>,
    ) -> Result<T, PersistenceError> {
        // A panic in another repository call leaves SQLite itself consistent,
        // so a poisoned lock is safe to reuse.
        let mut connection = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        work(&mut connection)
    }
}
