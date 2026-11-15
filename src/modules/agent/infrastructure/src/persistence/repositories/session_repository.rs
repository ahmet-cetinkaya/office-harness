// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::entities::Session;
use agent_domain::value_objects::{ExitResult, SessionId};
use kernel::{AgentId, Timestamp};
use rusqlite::{OptionalExtension, Row, params};

use crate::persistence::additions_document::AdditionsDocument;
use crate::persistence::{PersistenceError, SharedConnection};

const SELECT_SESSION: &str = "SELECT id, agent_id, cli_session_id, continued_from, \
                              additions_snapshot, started_at, ended_at, exit_result \
                              FROM agent_sessions";

pub struct SessionRepository {
    connection: SharedConnection,
}

struct SessionRow {
    id: String,
    agent_id: String,
    cli_session_id: Option<String>,
    continued_from: Option<String>,
    additions_snapshot: String,
    started_at: i64,
    ended_at: Option<i64>,
    exit_result: Option<String>,
}

impl SessionRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            agent_id: row.get(1)?,
            cli_session_id: row.get(2)?,
            continued_from: row.get(3)?,
            additions_snapshot: row.get(4)?,
            started_at: row.get(5)?,
            ended_at: row.get(6)?,
            exit_result: row.get(7)?,
        })
    }

    fn into_session(self) -> Result<Session, PersistenceError> {
        Ok(Session {
            id: self.id.parse()?,
            agent_id: self.agent_id.parse()?,
            cli_session_id: self.cli_session_id,
            continued_from: self.continued_from.map(|id| id.parse()).transpose()?,
            additions_snapshot: AdditionsDocument::parse(&self.additions_snapshot)?,
            started_at: Timestamp::from_unix_millis(self.started_at),
            ended_at: self.ended_at.map(Timestamp::from_unix_millis),
            exit_result: self.exit_result.map(|text| text.parse()).transpose()?,
        })
    }
}

impl SessionRepository {
    #[must_use]
    pub fn new(connection: SharedConnection) -> Self {
        Self { connection }
    }

    /// # Errors
    /// A storage failure, which includes an agent that does not exist.
    pub fn insert(&self, session: &Session) -> Result<(), PersistenceError> {
        let snapshot = AdditionsDocument::text_of(&session.additions_snapshot)?;
        self.connection.with(|connection| {
            connection.execute(
                "INSERT INTO agent_sessions (id, agent_id, cli_session_id, continued_from, \
                 additions_snapshot, started_at, ended_at, exit_result) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    session.id.to_string(),
                    session.agent_id.to_string(),
                    session.cli_session_id,
                    session.continued_from.map(|id| id.to_string()),
                    snapshot,
                    session.started_at.as_unix_millis(),
                    session.ended_at.map(Timestamp::as_unix_millis),
                    session.exit_result.as_ref().map(ToString::to_string),
                ],
            )?;
            Ok(())
        })
    }

    /// # Errors
    /// A storage failure or a stored row that the domain rejects.
    pub fn find(&self, id: SessionId) -> Result<Option<Session>, PersistenceError> {
        self.connection.with(|connection| {
            let row = connection
                .query_row(
                    &format!("{SELECT_SESSION} WHERE id = ?1"),
                    [id.to_string()],
                    SessionRow::read,
                )
                .optional()?;
            row.map(SessionRow::into_session).transpose()
        })
    }

    /// The agent's sessions, newest first.
    ///
    /// # Errors
    /// A storage failure or a stored row that the domain rejects.
    pub fn list_for_agent(&self, agent_id: AgentId) -> Result<Vec<Session>, PersistenceError> {
        self.connection.with(|connection| {
            let mut statement = connection.prepare(&format!(
                "{SELECT_SESSION} WHERE agent_id = ?1 ORDER BY started_at DESC, rowid DESC"
            ))?;
            let rows = statement
                .query_map([agent_id.to_string()], SessionRow::read)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows.into_iter().map(SessionRow::into_session).collect()
        })
    }

    /// # Errors
    /// A storage failure or a stored row that the domain rejects.
    pub fn latest_for_agent(&self, agent_id: AgentId) -> Result<Option<Session>, PersistenceError> {
        self.connection.with(|connection| {
            let row = connection
                .query_row(
                    &format!(
                        "{SELECT_SESSION} WHERE agent_id = ?1 \
                         ORDER BY started_at DESC, rowid DESC LIMIT 1"
                    ),
                    [agent_id.to_string()],
                    SessionRow::read,
                )
                .optional()?;
            row.map(SessionRow::into_session).transpose()
        })
    }

    /// # Errors
    /// `NotFound` when no session has the id.
    pub fn set_cli_session_id(
        &self,
        id: SessionId,
        cli_session_id: &str,
    ) -> Result<(), PersistenceError> {
        self.connection.with(|connection| {
            let changed = connection.execute(
                "UPDATE agent_sessions SET cli_session_id = ?2 WHERE id = ?1",
                params![id.to_string(), cli_session_id],
            )?;
            if changed == 0 {
                return Err(PersistenceError::NotFound {
                    what: format!("session {id}"),
                });
            }
            Ok(())
        })
    }

    /// Ends the agent's running session, if it has one.
    ///
    /// # Errors
    /// A storage failure.
    pub fn end_running(
        &self,
        agent_id: AgentId,
        result: &ExitResult,
        at: Timestamp,
    ) -> Result<bool, PersistenceError> {
        self.connection.with(|connection| {
            let changed = connection.execute(
                "UPDATE agent_sessions SET ended_at = ?2, exit_result = ?3 \
                 WHERE agent_id = ?1 AND ended_at IS NULL",
                params![
                    agent_id.to_string(),
                    at.as_unix_millis(),
                    result.to_string()
                ],
            )?;
            Ok(changed > 0)
        })
    }

    /// Ends every session that still runs as `interrupted` and returns how
    /// many changed. The server calls it once at start, when no session can
    /// be alive (FR-004).
    ///
    /// # Errors
    /// A storage failure.
    pub fn mark_running_as_interrupted(&self, at: Timestamp) -> Result<usize, PersistenceError> {
        self.connection.with(|connection| {
            Ok(connection.execute(
                "UPDATE agent_sessions SET ended_at = ?1, exit_result = ?2 \
                 WHERE ended_at IS NULL",
                params![at.as_unix_millis(), ExitResult::Interrupted.to_string()],
            )?)
        })
    }
}
