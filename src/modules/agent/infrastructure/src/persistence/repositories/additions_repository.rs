// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::entities::AdditionsRecord;
use kernel::{AgentId, Timestamp};
use rusqlite::{OptionalExtension, params};

use crate::persistence::additions_document::AdditionsDocument;
use crate::persistence::{PersistenceError, SharedConnection};

pub struct AdditionsRepository {
    connection: SharedConnection,
}

impl AdditionsRepository {
    #[must_use]
    pub fn new(connection: SharedConnection) -> Self {
        Self { connection }
    }

    /// # Errors
    /// A storage failure or a stored row that the domain rejects.
    pub fn load(&self, agent_id: AgentId) -> Result<Option<AdditionsRecord>, PersistenceError> {
        self.connection.with(|connection| {
            let row: Option<(String, i64)> = connection
                .query_row(
                    "SELECT additions, updated_at FROM agent_office_additions WHERE agent_id = ?1",
                    [agent_id.to_string()],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            row.map(|(document, updated_at)| {
                Ok(AdditionsRecord {
                    additions: AdditionsDocument::parse(&document)?,
                    updated_at: Timestamp::from_unix_millis(updated_at),
                })
            })
            .transpose()
        })
    }

    /// Replaces the agent's row.
    ///
    /// # Errors
    /// A storage failure, which includes an agent that does not exist.
    pub fn save(
        &self,
        agent_id: AgentId,
        record: &AdditionsRecord,
    ) -> Result<(), PersistenceError> {
        let document = AdditionsDocument::text_of(&record.additions)?;
        self.connection.with(|connection| {
            connection.execute(
                "INSERT INTO agent_office_additions (agent_id, additions, updated_at) \
                 VALUES (?1, ?2, ?3) \
                 ON CONFLICT (agent_id) DO UPDATE SET additions = ?2, updated_at = ?3",
                params![
                    agent_id.to_string(),
                    document,
                    record.updated_at.as_unix_millis()
                ],
            )?;
            Ok(())
        })
    }
}
