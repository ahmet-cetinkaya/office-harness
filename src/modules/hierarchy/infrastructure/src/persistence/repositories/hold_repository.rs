// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, PoisonError};

use hierarchy_application::common::errors::HierarchyApplicationError;
use hierarchy_application::common::interfaces::HoldStore;
use hierarchy_domain::{Hold, HoldScope, HoldSetter};
use kernel::{AgentId, Timestamp};
use rusqlite::{Connection, Row, params};

const OFFICE_SCOPE: &str = "office";
const USER_SETTER: &str = "user";
const OFFICE_SETTER: &str = "office";

/// SQLite storage of the holds, in the one database the host opened.
pub struct HoldRepository {
    connection: Arc<Mutex<Connection>>,
}

impl HoldRepository {
    #[must_use]
    pub fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }

    fn locked<T>(
        &self,
        work: impl FnOnce(&mut Connection) -> rusqlite::Result<T>,
    ) -> Result<T, HierarchyApplicationError> {
        // A panic in another call leaves SQLite itself consistent, so a
        // poisoned lock is safe to reuse.
        let mut connection = self
            .connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        work(&mut connection).map_err(|error| HierarchyApplicationError::Storage(error.to_string()))
    }
}

fn scope_text(scope: HoldScope) -> String {
    match scope {
        HoldScope::Office => OFFICE_SCOPE.to_owned(),
        HoldScope::Agent(agent_id) => agent_id.as_uuid_text(),
    }
}

fn read_hold(row: &Row<'_>) -> rusqlite::Result<Hold> {
    let scope: String = row.get(0)?;
    let set_by: String = row.get(1)?;
    let scope = if scope == OFFICE_SCOPE {
        HoldScope::Office
    } else {
        HoldScope::Agent(AgentId::from_uuid_text(&scope).ok_or(rusqlite::Error::InvalidQuery)?)
    };
    let set_by = match set_by.as_str() {
        USER_SETTER => HoldSetter::User,
        OFFICE_SETTER => HoldSetter::Office,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    Ok(Hold {
        scope,
        set_by,
        reason: row.get(2)?,
        since: Timestamp::from_unix_millis(row.get(3)?),
    })
}

impl HoldStore for HoldRepository {
    fn load_holds(&self) -> Result<Vec<Hold>, HierarchyApplicationError> {
        self.locked(|connection| {
            connection
                .prepare("SELECT scope, set_by, reason, since FROM hierarchy_holds ORDER BY since, rowid")?
                .query_map([], read_hold)?
                .collect()
        })
    }

    fn save_hold(&self, hold: &Hold) -> Result<(), HierarchyApplicationError> {
        let set_by = match hold.set_by {
            HoldSetter::User => USER_SETTER,
            HoldSetter::Office => OFFICE_SETTER,
        };
        self.locked(|connection| {
            connection
                .execute(
                    "INSERT OR REPLACE INTO hierarchy_holds (scope, set_by, reason, since) \
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        scope_text(hold.scope),
                        set_by,
                        hold.reason,
                        hold.since.as_unix_millis()
                    ],
                )
                .map(|_| ())
        })
    }

    fn delete_hold(&self, scope: HoldScope) -> Result<(), HierarchyApplicationError> {
        self.locked(|connection| {
            connection
                .execute(
                    "DELETE FROM hierarchy_holds WHERE scope = ?1",
                    params![scope_text(scope)],
                )
                .map(|_| ())
        })
    }

    fn delete_agent_holds(&self, agent_ids: &[AgentId]) -> Result<(), HierarchyApplicationError> {
        self.locked(|connection| {
            let transaction = connection.transaction()?;
            for agent_id in agent_ids {
                transaction.execute(
                    "DELETE FROM hierarchy_holds WHERE scope = ?1",
                    params![agent_id.as_uuid_text()],
                )?;
            }
            transaction.commit()
        })
    }
}
