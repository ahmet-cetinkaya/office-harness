// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::entities::Machine;
use agent_domain::value_objects::MachineId;
use rusqlite::{OptionalExtension, params};

use crate::persistence::{PersistenceError, SharedConnection};

pub struct MachineRepository {
    connection: SharedConnection,
}

impl MachineRepository {
    #[must_use]
    pub fn new(connection: SharedConnection) -> Self {
        Self { connection }
    }

    /// The office's one machine. The first call creates it with `host_name`.
    ///
    /// # Errors
    /// A storage failure, or a name or stored row that the domain rejects.
    pub fn ensure_local(&self, host_name: &str) -> Result<Machine, PersistenceError> {
        self.connection.with(|connection| {
            let stored: Option<(String, String)> = connection
                .query_row(
                    "SELECT id, name FROM agent_machines ORDER BY rowid LIMIT 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            if let Some((id, name)) = stored {
                return Ok(Machine::new(id.parse()?, &name)?);
            }
            let machine = Machine::new(MachineId::generate(), host_name)?;
            connection.execute(
                "INSERT INTO agent_machines (id, name) VALUES (?1, ?2)",
                params![machine.id.to_string(), machine.name],
            )?;
            Ok(machine)
        })
    }
}
