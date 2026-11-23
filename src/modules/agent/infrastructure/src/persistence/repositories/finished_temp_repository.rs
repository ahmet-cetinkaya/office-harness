// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use agent_domain::entities::{FinishedReport, FinishedTemp};
use agent_domain::value_objects::{FinishedTempId, creator_text, parse_creator};
use kernel::Timestamp;
use rusqlite::{Row, params};

use crate::persistence::{PersistenceError, SharedConnection};

const SELECT_FINISHED_TEMP: &str = "SELECT id, agent_id, name, objective, creator, parent_name, \
                                    started_at, ended_at, reason, report_kind, report_text, \
                                    kept_folder, kept_branch, removal_failure \
                                    FROM agent_finished_temps";

pub struct FinishedTempRepository {
    connection: SharedConnection,
}

struct FinishedTempRow {
    id: String,
    agent_id: String,
    name: String,
    objective: String,
    creator: String,
    parent_name: Option<String>,
    started_at: i64,
    ended_at: i64,
    reason: String,
    report_kind: Option<String>,
    report_text: Option<String>,
    kept_folder: Option<String>,
    kept_branch: Option<String>,
    removal_failure: Option<String>,
}

impl FinishedTempRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            agent_id: row.get(1)?,
            name: row.get(2)?,
            objective: row.get(3)?,
            creator: row.get(4)?,
            parent_name: row.get(5)?,
            started_at: row.get(6)?,
            ended_at: row.get(7)?,
            reason: row.get(8)?,
            report_kind: row.get(9)?,
            report_text: row.get(10)?,
            kept_folder: row.get(11)?,
            kept_branch: row.get(12)?,
            removal_failure: row.get(13)?,
        })
    }

    fn into_entry(self) -> Result<FinishedTemp, PersistenceError> {
        let creator = parse_creator(&self.creator).ok_or_else(|| {
            agent_domain::errors::AgentError::InvalidId {
                field: "temp creator",
                text: self.creator.clone(),
            }
        })?;
        Ok(FinishedTemp {
            id: self.id.parse()?,
            agent_id: self.agent_id.parse()?,
            name: self.name,
            objective: self.objective,
            creator,
            parent_name: self.parent_name,
            started_at: Timestamp::from_unix_millis(self.started_at),
            ended_at: Timestamp::from_unix_millis(self.ended_at),
            reason: self.reason,
            report: self
                .report_kind
                .zip(self.report_text)
                .map(|(kind, text)| FinishedReport { kind, text }),
            kept_folder: self.kept_folder.map(PathBuf::from),
            kept_branch: self.kept_branch,
            removal_failure: self.removal_failure,
        })
    }
}

impl FinishedTempRepository {
    #[must_use]
    pub fn new(connection: SharedConnection) -> Self {
        Self { connection }
    }

    /// # Errors
    /// A storage failure.
    pub fn insert(&self, entry: &FinishedTemp) -> Result<(), PersistenceError> {
        self.connection.with(|connection| {
            connection.execute(
                "INSERT INTO agent_finished_temps (id, agent_id, name, objective, creator, \
                 parent_name, started_at, ended_at, reason, report_kind, report_text, \
                 kept_folder, kept_branch, removal_failure) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    entry.id.to_string(),
                    entry.agent_id.to_string(),
                    entry.name,
                    entry.objective,
                    creator_text(entry.creator),
                    entry.parent_name,
                    entry.started_at.as_unix_millis(),
                    entry.ended_at.as_unix_millis(),
                    entry.reason,
                    entry.report.as_ref().map(|report| report.kind.as_str()),
                    entry.report.as_ref().map(|report| report.text.as_str()),
                    entry
                        .kept_folder
                        .as_ref()
                        .map(|folder| folder.to_string_lossy()),
                    entry.kept_branch,
                    entry.removal_failure,
                ],
            )?;
            Ok(())
        })
    }

    /// All entries, the one that ended last first.
    ///
    /// # Errors
    /// A storage failure or a stored row that the domain rejects.
    pub fn list(&self) -> Result<Vec<FinishedTemp>, PersistenceError> {
        self.connection.with(|connection| {
            let mut statement = connection.prepare(&format!(
                "{SELECT_FINISHED_TEMP} ORDER BY ended_at DESC, rowid DESC"
            ))?;
            let rows = statement
                .query_map([], FinishedTempRow::read)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows.into_iter().map(FinishedTempRow::into_entry).collect()
        })
    }

    /// Removes the one row. `false` when no entry has the id.
    ///
    /// # Errors
    /// A storage failure.
    pub fn dismiss(&self, id: FinishedTempId) -> Result<bool, PersistenceError> {
        self.connection.with(|connection| {
            let removed = connection.execute(
                "DELETE FROM agent_finished_temps WHERE id = ?1",
                [id.to_string()],
            )?;
            Ok(removed > 0)
        })
    }
}
