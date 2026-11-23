// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::Timestamp;
use rusqlite::params;

use crate::persistence::{PersistenceError, SharedConnection};

/// What `PRAGMA wal_checkpoint(TRUNCATE)` reports in its first column when
/// another connection kept it from finishing.
const CHECKPOINT_BUSY: i64 = 1;

/// How a truncating checkpoint of the write log ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteLogCheckpoint {
    Truncated,
    /// Another connection held a read transaction, so SQLite left the log.
    Busy,
}

/// The upkeep of the office database that the server runs at its start
/// (0023): the one-time rebuild and its mark, and the write log checkpoint.
pub struct MaintenanceRepository {
    connection: SharedConnection,
}

impl MaintenanceRepository {
    #[must_use]
    pub fn new(connection: SharedConnection) -> Self {
        Self { connection }
    }

    /// Whether the task has a done mark.
    ///
    /// # Errors
    /// A storage failure.
    pub fn is_done(&self, task: &str) -> Result<bool, PersistenceError> {
        self.connection.with(|connection| {
            Ok(connection.query_row(
                "SELECT EXISTS (SELECT 1 FROM agent_maintenance WHERE task = ?1)",
                [task],
                |row| row.get(0),
            )?)
        })
    }

    /// Writes the done mark of the task.
    ///
    /// # Errors
    /// A storage failure.
    pub fn mark_done(&self, task: &str, at: Timestamp) -> Result<(), PersistenceError> {
        self.connection.with(|connection| {
            connection.execute(
                "INSERT OR REPLACE INTO agent_maintenance (task, done_at) VALUES (?1, ?2)",
                params![task, at.as_unix_millis()],
            )?;
            Ok(())
        })
    }

    /// Rebuilds the database file, which drops the freed pages that old
    /// deletes left behind. It needs no open transaction on this connection.
    ///
    /// # Errors
    /// A storage failure, for example another connection that is writing.
    pub fn rebuild_file(&self) -> Result<(), PersistenceError> {
        self.connection.with(|connection| {
            connection.execute_batch("VACUUM")?;
            Ok(())
        })
    }

    /// Copies the write log into the database file and truncates the log, so
    /// no freed text stays in it.
    ///
    /// # Errors
    /// A storage failure.
    pub fn truncate_write_log(&self) -> Result<WriteLogCheckpoint, PersistenceError> {
        self.connection.with(|connection| {
            let busy: i64 =
                connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))?;
            Ok(if busy == CHECKPOINT_BUSY {
                WriteLogCheckpoint::Busy
            } else {
                WriteLogCheckpoint::Truncated
            })
        })
    }
}
