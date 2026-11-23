// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use kernel::{AgentId, Timestamp};
use rusqlite::ffi::ErrorCode;
use rusqlite::{Connection, OptionalExtension, Row, params};
use secret_application::{SecretStore, StoreError, StoredGrant, StoredSecret};
use secret_domain::{Custody, SealedRecord, SecretId, SecretName, SecretVersion};

/// One shared SQLite connection, the way the host hands the database to every
/// module (each module keeps its own tables).
pub type SharedHandle = Arc<Mutex<Connection>>;

const SELECT_SECRET: &str =
    "SELECT id, name, version, sealed, created_at, updated_at FROM secret_secrets";
const SELECT_GRANTS: &str = "SELECT agent_id, last_read_version, last_read_at \
                             FROM secret_grants WHERE secret_id = ?1 ORDER BY agent_id";

/// The secrets, grants, and custody mark in the `secret_` tables.
#[derive(Clone)]
pub struct SecretRepository {
    connection: SharedHandle,
}

impl SecretRepository {
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
}

struct SecretRow {
    id: String,
    name: String,
    version: u32,
    sealed: Vec<u8>,
    created_at: i64,
    updated_at: i64,
}

impl SecretRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            version: row.get(2)?,
            sealed: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    }

    /// The stored name is checked again by the domain rule, so a damaged row
    /// cannot give an invalid name to a request.
    fn into_secret(self, grants: Vec<StoredGrant>) -> Result<StoredSecret, StoreError> {
        let name = SecretName::parse(&self.name)
            .map_err(|error| corrupt(&format!("secret name: {error}")))?;
        Ok(StoredSecret {
            id: SecretId::from_stored(self.id),
            name,
            version: SecretVersion::from_stored(self.version),
            sealed: SealedRecord::from_bytes(self.sealed),
            created_at: Timestamp::from_unix_millis(self.created_at),
            updated_at: Timestamp::from_unix_millis(self.updated_at),
            grants,
        })
    }
}

fn corrupt(text: &str) -> StoreError {
    StoreError {
        message: format!("stored data is invalid, {text}"),
        is_disk_failure: false,
    }
}

fn failure(error: &rusqlite::Error) -> StoreError {
    let is_disk_failure = matches!(
        error.sqlite_error_code(),
        Some(
            ErrorCode::DiskFull
                | ErrorCode::SystemIoFailure
                | ErrorCode::ReadOnly
                | ErrorCode::CannotOpen
        )
    );
    StoreError {
        message: error.to_string(),
        is_disk_failure,
    }
}

fn read_grants(connection: &Connection, id: &str) -> Result<Vec<StoredGrant>, StoreError> {
    let mut statement = connection.prepare(SELECT_GRANTS).map_err(|e| failure(&e))?;
    let rows = statement
        .query_map(params![id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<u32>>(1)?,
                row.get::<_, Option<i64>>(2)?,
            ))
        })
        .map_err(|e| failure(&e))?;
    let mut grants = Vec::new();
    for row in rows {
        let (agent, version, at) = row.map_err(|e| failure(&e))?;
        let agent_id = agent
            .parse::<AgentId>()
            .map_err(|_| corrupt("grant agent id"))?;
        grants.push(StoredGrant {
            agent_id,
            last_read_version: version.map(SecretVersion::from_stored),
            last_read_at: at.map(Timestamp::from_unix_millis),
        });
    }
    Ok(grants)
}

fn load_secret(connection: &Connection, row: SecretRow) -> Result<StoredSecret, StoreError> {
    let grants = read_grants(connection, &row.id)?;
    row.into_secret(grants)
}

impl SecretStore for SecretRepository {
    fn list(&self) -> Result<Vec<StoredSecret>, StoreError> {
        let connection = self.connection();
        let mut statement = connection
            .prepare(&format!("{SELECT_SECRET} ORDER BY name"))
            .map_err(|e| failure(&e))?;
        let rows = statement
            .query_map([], SecretRow::read)
            .map_err(|e| failure(&e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| failure(&e))?;
        rows.into_iter()
            .map(|row| load_secret(&connection, row))
            .collect()
    }

    fn find(&self, name: &SecretName) -> Result<Option<StoredSecret>, StoreError> {
        let connection = self.connection();
        let row = connection
            .query_row(
                &format!("{SELECT_SECRET} WHERE name = ?1"),
                params![name.as_str()],
                SecretRow::read,
            )
            .optional()
            .map_err(|e| failure(&e))?;
        row.map(|row| load_secret(&connection, row)).transpose()
    }

    fn insert(&self, secret: &StoredSecret) -> Result<(), StoreError> {
        self.connection()
            .execute(
                "INSERT INTO secret_secrets \
                 (id, name, version, sealed, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    secret.id.as_str(),
                    secret.name.as_str(),
                    secret.version.number(),
                    secret.sealed.as_bytes(),
                    secret.created_at.as_unix_millis(),
                    secret.updated_at.as_unix_millis(),
                ],
            )
            .map_err(|e| failure(&e))?;
        Ok(())
    }

    fn replace(
        &self,
        id: &SecretId,
        version: SecretVersion,
        sealed: &SealedRecord,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        // One statement, so the record and its version change together.
        self.connection()
            .execute(
                "UPDATE secret_secrets SET version = ?2, sealed = ?3, updated_at = ?4 \
                 WHERE id = ?1",
                params![
                    id.as_str(),
                    version.number(),
                    sealed.as_bytes(),
                    at.as_unix_millis()
                ],
            )
            .map_err(|e| failure(&e))?;
        Ok(())
    }

    fn delete(&self, id: &SecretId) -> Result<(), StoreError> {
        // The grants go with the secret (ON DELETE CASCADE).
        self.connection()
            .execute(
                "DELETE FROM secret_secrets WHERE id = ?1",
                params![id.as_str()],
            )
            .map_err(|e| failure(&e))?;
        Ok(())
    }

    fn add_grant(&self, id: &SecretId, agent: &AgentId) -> Result<bool, StoreError> {
        let added = self
            .connection()
            .execute(
                "INSERT OR IGNORE INTO secret_grants (secret_id, agent_id) VALUES (?1, ?2)",
                params![id.as_str(), agent.as_uuid_text()],
            )
            .map_err(|e| failure(&e))?;
        Ok(added == 1)
    }

    fn remove_grant(&self, id: &SecretId, agent: &AgentId) -> Result<bool, StoreError> {
        let removed = self
            .connection()
            .execute(
                "DELETE FROM secret_grants WHERE secret_id = ?1 AND agent_id = ?2",
                params![id.as_str(), agent.as_uuid_text()],
            )
            .map_err(|e| failure(&e))?;
        Ok(removed == 1)
    }

    fn remove_grants_of_agent(&self, agent: &AgentId) -> Result<(), StoreError> {
        self.connection()
            .execute(
                "DELETE FROM secret_grants WHERE agent_id = ?1",
                params![agent.as_uuid_text()],
            )
            .map_err(|e| failure(&e))?;
        Ok(())
    }

    fn record_read(
        &self,
        id: &SecretId,
        agent: &AgentId,
        version: SecretVersion,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        self.connection()
            .execute(
                "UPDATE secret_grants SET last_read_version = ?3, last_read_at = ?4 \
                 WHERE secret_id = ?1 AND agent_id = ?2",
                params![
                    id.as_str(),
                    agent.as_uuid_text(),
                    version.number(),
                    at.as_unix_millis()
                ],
            )
            .map_err(|e| failure(&e))?;
        Ok(())
    }

    fn custody(&self) -> Result<Option<Custody>, StoreError> {
        let text: Option<String> = self
            .connection()
            .query_row("SELECT custody FROM secret_store WHERE id = 1", [], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|e| failure(&e))?;
        text.map(|text| Custody::parse(&text).ok_or_else(|| corrupt("custody")))
            .transpose()
    }

    fn set_custody(&self, custody: Custody) -> Result<(), StoreError> {
        self.connection()
            .execute(
                "INSERT INTO secret_store (id, custody) VALUES (1, ?1) \
                 ON CONFLICT (id) DO UPDATE SET custody = excluded.custody",
                params![custody.as_str()],
            )
            .map_err(|e| failure(&e))?;
        Ok(())
    }

    fn reset(&self) -> Result<usize, StoreError> {
        let mut connection = self.connection();
        let transaction = connection.transaction().map_err(|e| failure(&e))?;
        let removed = transaction
            .execute("DELETE FROM secret_secrets", [])
            .map_err(|e| failure(&e))?;
        transaction
            .execute("DELETE FROM secret_grants", [])
            .map_err(|e| failure(&e))?;
        transaction
            .execute("DELETE FROM secret_store", [])
            .map_err(|e| failure(&e))?;
        transaction.commit().map_err(|e| failure(&e))?;
        Ok(removed)
    }
}
