// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{MutexGuard, PoisonError};

use kernel::{AgentId, Timestamp};
use notification_application::common::errors::NotificationApplicationError;
use notification_application::common::interfaces::NeedsYouStore;
use notification_domain::{
    AttentionEvent, AttentionKind, ItemReport, NeedsYouItem, NotificationId,
};
use rusqlite::{Connection, OptionalExtension, Row, params};

use super::notification_repository::{SharedHandle, corrupt, storage_failure};

const SELECT_ITEM: &str = "SELECT agent_id, first_event_at, latest_kind, latest_detail, \
                           latest_at, origin, report_kind, report_text, report_at, waits_only \
                           FROM notification_needs_you";

struct NeedsYouRow {
    agent_id: String,
    first_event_at: i64,
    latest_kind: String,
    latest_detail: Option<String>,
    latest_at: i64,
    origin: Option<String>,
    report_kind: Option<String>,
    report_text: Option<String>,
    report_at: Option<i64>,
    waits_only: bool,
}

impl NeedsYouRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            agent_id: row.get(0)?,
            first_event_at: row.get(1)?,
            latest_kind: row.get(2)?,
            latest_detail: row.get(3)?,
            latest_at: row.get(4)?,
            origin: row.get(5)?,
            report_kind: row.get(6)?,
            report_text: row.get(7)?,
            report_at: row.get(8)?,
            waits_only: row.get(9)?,
        })
    }

    fn into_item(self) -> Result<NeedsYouItem, NotificationApplicationError> {
        let agent_id = self
            .agent_id
            .parse::<AgentId>()
            .map_err(|error| corrupt("agent id", &error.to_string()))?;
        let kind = self
            .latest_kind
            .parse::<AttentionKind>()
            .map_err(|error| corrupt("attention kind", &error.to_string()))?;
        let origin = self
            .origin
            .map(|text| {
                text.parse::<NotificationId>()
                    .map_err(|error| corrupt("origin notification id", &error.to_string()))
            })
            .transpose()?;
        // A row holds a report only when all three columns are set; a row
        // written before 0015 has none.
        let report = match (self.report_kind, self.report_text, self.report_at) {
            (Some(kind), Some(text), Some(at)) => Some(ItemReport {
                kind,
                text,
                at: Timestamp::from_unix_millis(at),
            }),
            _ => None,
        };
        Ok(NeedsYouItem {
            agent_id,
            first_event_at: Timestamp::from_unix_millis(self.first_event_at),
            latest: AttentionEvent {
                kind,
                detail: self.latest_detail,
                at: Timestamp::from_unix_millis(self.latest_at),
            },
            origin,
            report,
            waits_only: self.waits_only,
        })
    }
}

/// The items in the `notification_needs_you` table.
#[derive(Clone)]
pub struct NeedsYouRepository {
    connection: SharedHandle,
}

impl NeedsYouRepository {
    #[must_use]
    pub fn new(connection: SharedHandle) -> Self {
        Self { connection }
    }

    fn connection(&self) -> MutexGuard<'_, Connection> {
        // A panic in another call leaves SQLite consistent, so a poisoned
        // lock is safe to reuse.
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn delete_where(
        &self,
        column: &'static str,
        value: String,
    ) -> Result<bool, NotificationApplicationError> {
        let deleted = self
            .connection()
            .execute(
                &format!("DELETE FROM notification_needs_you WHERE {column} = ?1"),
                [value],
            )
            .map_err(|error| storage_failure(&error))?;
        Ok(deleted > 0)
    }
}

impl NeedsYouStore for NeedsYouRepository {
    fn find(
        &self,
        agent_id: AgentId,
    ) -> Result<Option<NeedsYouItem>, NotificationApplicationError> {
        let row = self
            .connection()
            .query_row(
                &format!("{SELECT_ITEM} WHERE agent_id = ?1"),
                [agent_id.to_string()],
                NeedsYouRow::read,
            )
            .optional()
            .map_err(|error| storage_failure(&error))?;
        row.map(NeedsYouRow::into_item).transpose()
    }

    fn save(&self, item: &NeedsYouItem) -> Result<(), NotificationApplicationError> {
        self.connection()
            .execute(
                "INSERT OR REPLACE INTO notification_needs_you \
                 (agent_id, first_event_at, latest_kind, latest_detail, latest_at, origin, \
                 report_kind, report_text, report_at, waits_only) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    item.agent_id.to_string(),
                    item.first_event_at.as_unix_millis(),
                    item.latest.kind.as_str(),
                    item.latest.detail,
                    item.latest.at.as_unix_millis(),
                    item.origin.map(|origin| origin.as_uuid_text()),
                    item.report.as_ref().map(|report| report.kind.as_str()),
                    item.report.as_ref().map(|report| report.text.as_str()),
                    item.report
                        .as_ref()
                        .map(|report| report.at.as_unix_millis()),
                    item.waits_only,
                ],
            )
            .map_err(|error| storage_failure(&error))?;
        Ok(())
    }

    fn list(&self) -> Result<Vec<NeedsYouItem>, NotificationApplicationError> {
        let connection = self.connection();
        let mut statement = connection
            .prepare(SELECT_ITEM)
            .map_err(|error| storage_failure(&error))?;
        let rows = statement
            .query_map([], NeedsYouRow::read)
            .map_err(|error| storage_failure(&error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| storage_failure(&error))?;
        rows.into_iter().map(NeedsYouRow::into_item).collect()
    }

    fn remove(&self, agent_id: AgentId) -> Result<bool, NotificationApplicationError> {
        self.delete_where("agent_id", agent_id.to_string())
    }

    fn remove_by_origin(
        &self,
        notification_id: NotificationId,
    ) -> Result<bool, NotificationApplicationError> {
        self.delete_where("origin", notification_id.as_uuid_text())
    }
}
