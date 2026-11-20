// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, PoisonError};

use kernel::{AgentId, Timestamp};
use notification_application::common::errors::NotificationApplicationError;
use notification_application::queries::list_notifications::LIST_LIMIT;
use notification_domain::{Notification, NotificationEvent, NotificationId};
use rusqlite::ffi::ErrorCode;
use rusqlite::{Connection, OptionalExtension, Row, params};

/// One shared SQLite connection, the way the host app hands the database to
/// every module (each module keeps its own tables).
pub type SharedHandle = Arc<Mutex<Connection>>;

const SELECT_NOTIFICATION: &str = "SELECT id, agent_id, agent_name, event, created_at, read_at, detail \
                                   FROM notification_notifications";

/// The order of every listing (D4): unread first, then newest first. The
/// `rowid` breaks the ties of one created millisecond by insertion, newest
/// insert first, so the order never depends on the random ids.
///
/// The unread-first term is an expression over the read state, and no index
/// on this table covers that expression, so no index serves this `ORDER BY`
/// and SQLite scans the table once and sorts. The [`LIST_LIMIT`] cap bounds the
/// sort: it keeps only the top rows, so the memory of the listing grows with
/// the cap, not with the table.
const LIST_ORDER: &str = "ORDER BY read_at IS NOT NULL, created_at DESC, rowid DESC";

/// The newest rows only, without the read ones.
const LIST_UNREAD_ORDER: &str = "WHERE read_at IS NULL ORDER BY created_at DESC, rowid DESC";

struct NotificationRow {
    id: String,
    agent_id: String,
    agent_name: String,
    event: String,
    created_at: i64,
    read_at: Option<i64>,
    detail: Option<String>,
}

impl NotificationRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            agent_id: row.get(1)?,
            agent_name: row.get(2)?,
            event: row.get(3)?,
            created_at: row.get(4)?,
            read_at: row.get(5)?,
            detail: row.get(6)?,
        })
    }

    fn into_notification(self) -> Result<Notification, NotificationApplicationError> {
        let id = self
            .id
            .parse::<NotificationId>()
            .map_err(|error| corrupt("notification id", &error.to_string()))?;
        let agent_id = self
            .agent_id
            .parse::<AgentId>()
            .map_err(|error| corrupt("agent id", &error.to_string()))?;
        Ok(Notification {
            id,
            agent_id,
            agent_name: self.agent_name,
            event: self
                .event
                .parse::<NotificationEvent>()
                .map_err(|error| corrupt("notification event", &error.to_string()))?,
            created_at: Timestamp::from_unix_millis(self.created_at),
            read_at: self.read_at.map(Timestamp::from_unix_millis),
            detail: self.detail,
        })
    }
}

pub(super) fn corrupt(field: &'static str, text: &str) -> NotificationApplicationError {
    NotificationApplicationError::Storage {
        message: format!("stored data is invalid, {field}: {text}"),
        is_disk_failure: false,
    }
}

pub(super) fn storage_failure(error: &rusqlite::Error) -> NotificationApplicationError {
    let is_disk_failure = matches!(
        error.sqlite_error_code(),
        Some(
            ErrorCode::DiskFull
                | ErrorCode::SystemIoFailure
                | ErrorCode::ReadOnly
                | ErrorCode::CannotOpen
        )
    );
    NotificationApplicationError::Storage {
        message: error.to_string(),
        is_disk_failure,
    }
}

/// The notifications in the `notification_notifications` table.
#[derive(Clone)]
pub struct NotificationRepository {
    connection: SharedHandle,
}

impl NotificationRepository {
    #[must_use]
    pub fn new(connection: SharedHandle) -> Self {
        Self { connection }
    }

    fn connection(&self) -> std::sync::MutexGuard<'_, Connection> {
        // A panic in another call leaves SQLite consistent, so a poisoned
        // lock is safe to reuse.
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn store(&self, notification: &Notification) -> Result<(), NotificationApplicationError> {
        self.connection()
            .execute(
                "INSERT INTO notification_notifications \
                 (id, agent_id, agent_name, event, created_at, read_at, detail) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    notification.id.as_uuid_text(),
                    notification.agent_id.to_string(),
                    notification.agent_name,
                    notification.event.as_str(),
                    notification.created_at.as_unix_millis(),
                    notification.read_at.map(kernel::Timestamp::as_unix_millis),
                    notification.detail,
                ],
            )
            .map_err(|error| storage_failure(&error))?;
        Ok(())
    }
}

impl NotificationRepository {
    /// # Errors
    /// A storage failure or a stored row that the domain rejects.
    pub fn find(
        &self,
        notification_id: NotificationId,
    ) -> Result<Option<Notification>, NotificationApplicationError> {
        let connection = self.connection();
        let row = connection
            .query_row(
                &format!("{SELECT_NOTIFICATION} WHERE id = ?1"),
                [notification_id.as_uuid_text()],
                NotificationRow::read,
            )
            .optional()
            .map_err(|error| storage_failure(&error))?;
        row.map(NotificationRow::into_notification).transpose()
    }

    /// # Errors
    /// A storage failure or a stored row that the domain rejects.
    pub fn update(&self, notification: &Notification) -> Result<(), NotificationApplicationError> {
        self.connection()
            .execute(
                "UPDATE notification_notifications SET agent_name = ?2, event = ?3, \
                 created_at = ?4, read_at = ?5 WHERE id = ?1",
                params![
                    notification.id.as_uuid_text(),
                    notification.agent_name,
                    notification.event.as_str(),
                    notification.created_at.as_unix_millis(),
                    notification.read_at.map(kernel::Timestamp::as_unix_millis),
                ],
            )
            .map_err(|error| storage_failure(&error))?;
        Ok(())
    }

    /// The unread rows of one agent, oldest first, so a read-by-agent stamps
    /// them in creation order.
    ///
    /// # Errors
    /// A storage failure or a stored row that the domain rejects.
    pub fn list_unread_for_agent(
        &self,
        agent_id: AgentId,
    ) -> Result<Vec<Notification>, NotificationApplicationError> {
        let connection = self.connection();
        let mut statement = connection
            .prepare(&format!(
                "{SELECT_NOTIFICATION} WHERE agent_id = ?1 AND read_at IS NULL ORDER BY created_at"
            ))
            .map_err(|error| storage_failure(&error))?;
        let rows = statement
            .query_map([agent_id.to_string()], NotificationRow::read)
            .map_err(|error| storage_failure(&error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| storage_failure(&error))?;
        rows.into_iter()
            .map(NotificationRow::into_notification)
            .collect()
    }
}

impl NotificationRepository {
    /// # Errors
    /// A storage failure.
    pub fn insert(&self, notification: &Notification) -> Result<(), NotificationApplicationError> {
        self.store(notification)
    }

    /// The newest rows, unread first, then newest first, at most
    /// [`LIST_LIMIT`] of them: the cap and the order are the query's, so a
    /// growing table is never loaded whole.
    ///
    /// # Errors
    /// A storage failure or a stored row that the domain rejects.
    pub fn list(&self) -> Result<Vec<Notification>, NotificationApplicationError> {
        self.list_capped(LIST_ORDER)
    }

    /// The newest unread rows of the whole office, at most [`LIST_LIMIT`] of
    /// them: the unread filter, the order, and the cap are the query's.
    ///
    /// # Errors
    /// A storage failure or a stored row that the domain rejects.
    pub fn list_unread(&self) -> Result<Vec<Notification>, NotificationApplicationError> {
        self.list_capped(LIST_UNREAD_ORDER)
    }

    /// Runs one capped listing: the tail joins the filter and order, the cap
    /// is the shared one.
    fn list_capped(&self, tail: &str) -> Result<Vec<Notification>, NotificationApplicationError> {
        let connection = self.connection();
        let mut statement = connection
            .prepare(&format!("{SELECT_NOTIFICATION} {tail} LIMIT {LIST_LIMIT}"))
            .map_err(|error| storage_failure(&error))?;
        let rows = statement
            .query_map([], NotificationRow::read)
            .map_err(|error| storage_failure(&error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| storage_failure(&error))?;
        rows.into_iter()
            .map(NotificationRow::into_notification)
            .collect()
    }
}
