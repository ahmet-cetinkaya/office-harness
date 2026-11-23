// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::models::{ProcessKind, SessionProcess, StoredProcess};
use agent_domain::value_objects::{ExitResult, SessionId};
use rusqlite::types::Type;
use rusqlite::{Connection, Row, params};

use crate::persistence::{PersistenceError, SharedConnection};

const SELECT_PROCESS: &str = "SELECT id, session_id, pid, started_at, command_hash, \
                              command_display, boot, kind FROM agent_session_processes";

const KIND_CLI: &str = "cli";
const KIND_HELPER: &str = "helper";

/// The stored identities of the processes of sessions (0017).
pub struct SessionProcessRepository {
    connection: SharedConnection,
}

impl SessionProcessRepository {
    #[must_use]
    pub fn new(connection: SharedConnection) -> Self {
        Self { connection }
    }

    /// Stores the process as open, unless the session already holds a row for
    /// this process id and start time or has ended. Returns whether a row was
    /// stored.
    ///
    /// # Errors
    /// A storage failure, or a start time that does not fit the database.
    pub fn insert_open(
        &self,
        session_id: SessionId,
        kind: ProcessKind,
        process: &SessionProcess,
    ) -> Result<bool, PersistenceError> {
        let started_at = integer_of(process.started_at)?;
        self.connection.with(|connection| {
            let inserted = connection.execute(
                // The WHERE clause also settles SQLite's parse of an upsert
                // after a SELECT, and keeps a session that just ended from
                // getting an open row.
                "INSERT INTO agent_session_processes \
                 (session_id, pid, started_at, command_hash, command_display, boot, kind, state) \
                 SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, 'open' \
                 WHERE EXISTS (SELECT 1 FROM agent_sessions WHERE id = ?1 AND ended_at IS NULL) \
                 ON CONFLICT (session_id, pid, started_at) DO NOTHING",
                params![
                    session_id.to_string(),
                    process.pid,
                    started_at,
                    process.command_hash.as_slice(),
                    process.command_display,
                    process.boot,
                    kind_text(kind),
                ],
            )?;
            Ok(inserted > 0)
        })
    }

    /// The open rows of one session.
    ///
    /// # Errors
    /// A storage failure or a stored row that does not read back.
    pub fn open_for_session(
        &self,
        session_id: SessionId,
    ) -> Result<Vec<StoredProcess>, PersistenceError> {
        self.connection.with(|connection| {
            open_rows(
                connection,
                "WHERE state = 'open' AND session_id = ?1 ORDER BY id",
                &session_id.to_string(),
            )
        })
    }

    /// The open rows of every session that ended as `lost`: the processes
    /// that an unclean stop may have left running.
    ///
    /// # Errors
    /// A storage failure or a stored row that does not read back.
    pub fn open_for_lost_sessions(&self) -> Result<Vec<StoredProcess>, PersistenceError> {
        self.connection.with(|connection| {
            open_rows(
                connection,
                "WHERE state = 'open' AND session_id IN \
                 (SELECT id FROM agent_sessions WHERE exit_result = ?1) ORDER BY id",
                &ExitResult::Lost.to_string(),
            )
        })
    }

    /// Replaces the command line of a row, which stays open.
    ///
    /// # Errors
    /// A storage failure.
    pub fn update_command(
        &self,
        id: i64,
        process: &SessionProcess,
    ) -> Result<(), PersistenceError> {
        self.connection.with(|connection| {
            connection.execute(
                "UPDATE agent_session_processes SET command_hash = ?2, command_display = ?3 \
                 WHERE id = ?1",
                params![id, process.command_hash.as_slice(), process.command_display],
            )?;
            Ok(())
        })
    }

    /// Deletes a helper row. A CLI row stays for the end of its session.
    ///
    /// # Errors
    /// A storage failure.
    pub fn drop_helper(&self, id: i64) -> Result<(), PersistenceError> {
        self.connection.with(|connection| {
            connection.execute(
                "DELETE FROM agent_session_processes WHERE id = ?1 AND kind = ?2",
                params![id, KIND_HELPER],
            )?;
            Ok(())
        })
    }

    /// Marks one row as ended, so no later sweep ends or records it.
    ///
    /// # Errors
    /// A storage failure.
    pub fn end_one(&self, id: i64) -> Result<(), PersistenceError> {
        self.connection
            .with(|connection| Self::end_one_in(connection, id))
    }

    /// Marks one row as ended on the caller's connection or transaction, so
    /// the sweep can write it in one transaction with its action record
    /// entry (0017/FR-009).
    ///
    /// # Errors
    /// A storage failure.
    pub fn end_one_in(connection: &Connection, id: i64) -> Result<(), PersistenceError> {
        connection.execute(
            "UPDATE agent_session_processes SET state = 'ended' WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    /// Marks every open row of the session as ended.
    ///
    /// # Errors
    /// A storage failure.
    pub fn end_for_session(&self, session_id: SessionId) -> Result<(), PersistenceError> {
        self.connection.with(|connection| {
            end_open_rows(connection, &session_id.to_string())?;
            Ok(())
        })
    }

    /// Marks the session as having left helpers unstored (the helper limit).
    ///
    /// # Errors
    /// A storage failure.
    pub fn mark_helper_limit(&self, session_id: SessionId) -> Result<(), PersistenceError> {
        self.connection.with(|connection| {
            connection.execute(
                "UPDATE agent_sessions SET helpers_capped = 1 WHERE id = ?1",
                [session_id.to_string()],
            )?;
            Ok(())
        })
    }
}

/// Marks every open row of the session as ended. A session end calls it in
/// the same transaction that ends the session row (0017/FR-008).
pub(super) fn end_open_rows(connection: &Connection, session_id: &str) -> rusqlite::Result<usize> {
    connection.execute(
        "UPDATE agent_session_processes SET state = 'ended' \
         WHERE session_id = ?1 AND state = 'open'",
        [session_id],
    )
}

fn open_rows(
    connection: &Connection,
    condition: &str,
    parameter: &str,
) -> Result<Vec<StoredProcess>, PersistenceError> {
    let mut statement = connection.prepare(&format!("{SELECT_PROCESS} {condition}"))?;
    let rows = statement
        .query_map([parameter], read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter().map(StoredRow::into_stored).collect()
}

struct StoredRow {
    id: i64,
    session_id: String,
    kind: ProcessKind,
    process: SessionProcess,
}

impl StoredRow {
    fn into_stored(self) -> Result<StoredProcess, PersistenceError> {
        Ok(StoredProcess {
            id: self.id,
            session_id: self.session_id.parse()?,
            kind: self.kind,
            process: self.process,
        })
    }
}

fn read_row(row: &Row<'_>) -> rusqlite::Result<StoredRow> {
    let command_hash: Vec<u8> = row.get(4)?;
    let kind: String = row.get(7)?;
    Ok(StoredRow {
        id: row.get(0)?,
        session_id: row.get(1)?,
        kind: kind_of(&kind).ok_or_else(|| invalid_column(7, Type::Text, "process kind"))?,
        process: SessionProcess {
            pid: row.get(2)?,
            started_at: u64::try_from(row.get::<_, i64>(3)?)
                .map_err(|_| invalid_column(3, Type::Integer, "start time"))?,
            command_hash: command_hash
                .try_into()
                .map_err(|_| invalid_column(4, Type::Blob, "command hash"))?,
            command_display: row.get(5)?,
            boot: row.get(6)?,
        },
    })
}

fn invalid_column(index: usize, kind: Type, what: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(index, kind, format!("invalid {what}").into())
}

fn integer_of(started_at: u64) -> Result<i64, PersistenceError> {
    i64::try_from(started_at).map_err(|error| {
        PersistenceError::Storage(rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
    })
}

fn kind_text(kind: ProcessKind) -> &'static str {
    match kind {
        ProcessKind::Cli => KIND_CLI,
        ProcessKind::Helper => KIND_HELPER,
    }
}

fn kind_of(text: &str) -> Option<ProcessKind> {
    match text {
        KIND_CLI => Some(ProcessKind::Cli),
        KIND_HELPER => Some(ProcessKind::Helper),
        _ => None,
    }
}
