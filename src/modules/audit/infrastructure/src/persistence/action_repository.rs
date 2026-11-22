// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, PoisonError};

use audit_application::common::errors::AuditError;
use audit_application::common::interfaces::ActionLog;
use audit_domain::{ActionEntry, ActionKind, ActionRecord, ActionResult};
use kernel::{Actor, AgentId, Timestamp};
use rusqlite::ffi::ErrorCode;
use rusqlite::{Connection, Row, params};

const USER_ACTOR: &str = "user";
const OFFICE_ACTOR: &str = "office";
const AGENT_ACTOR_PREFIX: &str = "agent:";
/// The newest office-level entries one query returns.
const OFFICE_LEVEL_LIMIT: i64 = 500;

/// The action record in the `audit_actions` table. The connection is the one
/// the host app opened on the office database.
#[derive(Clone)]
pub struct ActionRepository {
    connection: Arc<Mutex<Connection>>,
}

struct ActionRow {
    id: i64,
    actor: String,
    target_agent_id: Option<String>,
    action: String,
    result: String,
    at: i64,
    detail: Option<String>,
}

impl ActionRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            actor: row.get(1)?,
            target_agent_id: row.get(2)?,
            action: row.get(3)?,
            result: row.get(4)?,
            at: row.get(5)?,
            detail: row.get(6)?,
        })
    }

    fn into_entry(self) -> Result<ActionEntry, AuditError> {
        Ok(ActionEntry {
            id: self.id,
            record: ActionRecord {
                actor: parse_actor(&self.actor)?,
                target_agent_id: self
                    .target_agent_id
                    .as_deref()
                    .map(|text| text.parse().map_err(|_| corrupt("target agent id", text)))
                    .transpose()?,
                action: self.action.parse::<ActionKind>()?,
                result: ActionResult::from_text(&self.result),
                at: Timestamp::from_unix_millis(self.at),
                detail: self.detail,
            },
        })
    }
}

fn corrupt(field: &'static str, text: &str) -> AuditError {
    AuditError::Corrupt(audit_domain::ActionError {
        field,
        text: text.to_owned(),
    })
}

fn actor_text(actor: Actor) -> String {
    match actor {
        Actor::User => USER_ACTOR.to_owned(),
        Actor::Office => OFFICE_ACTOR.to_owned(),
        Actor::Agent(agent_id) => format!("{AGENT_ACTOR_PREFIX}{agent_id}"),
    }
}

fn parse_actor(text: &str) -> Result<Actor, AuditError> {
    match text {
        USER_ACTOR => return Ok(Actor::User),
        OFFICE_ACTOR => return Ok(Actor::Office),
        _ => {}
    }
    text.strip_prefix(AGENT_ACTOR_PREFIX)
        .and_then(|id| id.parse().ok())
        .map(Actor::Agent)
        .ok_or_else(|| corrupt("actor", text))
}

fn storage_failure(error: &rusqlite::Error) -> AuditError {
    let is_disk_failure = matches!(
        error.sqlite_error_code(),
        Some(
            ErrorCode::DiskFull
                | ErrorCode::SystemIoFailure
                | ErrorCode::ReadOnly
                | ErrorCode::CannotOpen
        )
    );
    AuditError::Storage {
        message: error.to_string(),
        is_disk_failure,
    }
}

impl ActionRepository {
    #[must_use]
    pub fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }

    fn connection(&self) -> std::sync::MutexGuard<'_, Connection> {
        // A panic in another call leaves SQLite consistent, so a poisoned
        // lock is safe to reuse.
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl ActionLog for ActionRepository {
    fn append(&self, record: &ActionRecord) -> Result<ActionEntry, AuditError> {
        let connection = self.connection();
        connection
            .execute(
                "INSERT INTO audit_actions (actor, target_agent_id, action, result, at, detail) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    actor_text(record.actor),
                    record.target_agent_id.map(|agent_id| agent_id.to_string()),
                    record.action.as_str(),
                    record.result.as_text(),
                    record.at.as_unix_millis(),
                    record.detail.as_deref(),
                ],
            )
            .map_err(|error| storage_failure(&error))?;
        Ok(ActionEntry {
            id: connection.last_insert_rowid(),
            record: record.clone(),
        })
    }

    fn list_for_agent(&self, agent_id: AgentId) -> Result<Vec<ActionEntry>, AuditError> {
        let connection = self.connection();
        let mut statement = connection
            .prepare(
                "SELECT id, actor, target_agent_id, action, result, at, detail FROM audit_actions \
                 WHERE target_agent_id = ?1 ORDER BY id",
            )
            .map_err(|error| storage_failure(&error))?;
        let rows = statement
            .query_map([agent_id.to_string()], ActionRow::read)
            .map_err(|error| storage_failure(&error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| storage_failure(&error))?;
        rows.into_iter().map(ActionRow::into_entry).collect()
    }

    fn list_office_level(&self) -> Result<Vec<ActionEntry>, AuditError> {
        let connection = self.connection();
        let mut statement = connection
            .prepare(
                "SELECT id, actor, target_agent_id, action, result, at, detail FROM audit_actions \
                 WHERE target_agent_id IS NULL ORDER BY id DESC LIMIT ?1",
            )
            .map_err(|error| storage_failure(&error))?;
        let rows = statement
            .query_map([OFFICE_LEVEL_LIMIT], ActionRow::read)
            .map_err(|error| storage_failure(&error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| storage_failure(&error))?;
        rows.into_iter().map(ActionRow::into_entry).collect()
    }
}

#[cfg(test)]
#[path = "action_repository.tests.rs"]
mod tests;
