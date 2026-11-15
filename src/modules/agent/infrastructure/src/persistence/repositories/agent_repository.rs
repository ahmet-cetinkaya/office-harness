// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use agent_domain::entities::Agent;
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{AgentCommand, DisplayName};
use kernel::{AgentId, Timestamp};
use rusqlite::{OptionalExtension, Row, params};

use crate::persistence::{PersistenceError, SharedConnection};

const SELECT_AGENT: &str = "SELECT id, name, command, working_folder, project_id, machine_id, \
                            status, created_at FROM agent_agents";

pub struct AgentRepository {
    connection: SharedConnection,
}

struct AgentRow {
    id: String,
    name: String,
    command: String,
    working_folder: Option<String>,
    project_id: Option<String>,
    machine_id: String,
    status: String,
    created_at: i64,
}

impl AgentRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            command: row.get(2)?,
            working_folder: row.get(3)?,
            project_id: row.get(4)?,
            machine_id: row.get(5)?,
            status: row.get(6)?,
            created_at: row.get(7)?,
        })
    }

    fn into_agent(self) -> Result<Agent, PersistenceError> {
        let mut agent = Agent::new(
            self.id.parse()?,
            DisplayName::parse("agent name", &self.name)?,
            AgentCommand::parse(&self.command)?,
            self.working_folder.map(PathBuf::from),
            self.project_id.map(|id| id.parse()).transpose()?,
            self.machine_id.parse()?,
            Timestamp::from_unix_millis(self.created_at),
        )?;
        agent.status = self.status.parse()?;
        Ok(agent)
    }
}

impl AgentRepository {
    #[must_use]
    pub fn new(connection: SharedConnection) -> Self {
        Self { connection }
    }

    /// # Errors
    /// `Duplicate` when another agent has the name, ignoring case.
    pub fn insert(&self, agent: &Agent) -> Result<(), PersistenceError> {
        self.connection.with(|connection| {
            connection
                .execute(
                    "INSERT INTO agent_agents (id, name, name_key, command, working_folder, \
                     project_id, machine_id, status, created_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        agent.id.to_string(),
                        agent.name.as_str(),
                        agent.name.uniqueness_key(),
                        agent.command.as_str(),
                        agent
                            .working_folder
                            .as_ref()
                            .map(|folder| folder.to_string_lossy()),
                        agent.project_id.map(|id| id.to_string()),
                        agent.machine_id.to_string(),
                        agent.status.as_str(),
                        agent.created_at.as_unix_millis(),
                    ],
                )
                .map_err(|error| {
                    PersistenceError::from_insert(error, format!("agent {}", agent.name))
                })?;
            Ok(())
        })
    }

    /// # Errors
    /// A storage failure or a stored row that the domain rejects.
    pub fn find(&self, id: AgentId) -> Result<Option<Agent>, PersistenceError> {
        self.connection.with(|connection| {
            let row = connection
                .query_row(
                    &format!("{SELECT_AGENT} WHERE id = ?1"),
                    [id.to_string()],
                    AgentRow::read,
                )
                .optional()?;
            row.map(AgentRow::into_agent).transpose()
        })
    }

    /// All agents, oldest first.
    ///
    /// # Errors
    /// A storage failure or a stored row that the domain rejects.
    pub fn list(&self) -> Result<Vec<Agent>, PersistenceError> {
        self.connection.with(|connection| {
            let mut statement =
                connection.prepare(&format!("{SELECT_AGENT} ORDER BY created_at, rowid"))?;
            let rows = statement
                .query_map([], AgentRow::read)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows.into_iter().map(AgentRow::into_agent).collect()
        })
    }

    /// # Errors
    /// `NotFound` when no agent has the id.
    pub fn update_status(&self, id: AgentId, status: AgentStatus) -> Result<(), PersistenceError> {
        self.connection.with(|connection| {
            let changed = connection.execute(
                "UPDATE agent_agents SET status = ?2 WHERE id = ?1",
                params![id.to_string(), status.as_str()],
            )?;
            require_one_row(changed, id)
        })
    }

    /// # Errors
    /// `NotFound` when no agent has the id.
    pub fn delete(&self, id: AgentId) -> Result<(), PersistenceError> {
        self.connection.with(|connection| {
            let changed =
                connection.execute("DELETE FROM agent_agents WHERE id = ?1", [id.to_string()])?;
            require_one_row(changed, id)
        })
    }

    /// Marks every agent that still shows a running status as `interrupted`
    /// and returns how many changed. The server calls it once at start, when no
    /// session can be alive (FR-004).
    ///
    /// # Errors
    /// A storage failure.
    pub fn mark_running_as_interrupted(&self) -> Result<usize, PersistenceError> {
        self.connection.with(|connection| {
            let running = AgentStatus::RUNNING.map(AgentStatus::as_str);
            let changed = connection.execute(
                "UPDATE agent_agents SET status = ?4 WHERE status IN (?1, ?2, ?3)",
                params![
                    running[0],
                    running[1],
                    running[2],
                    AgentStatus::Interrupted.as_str()
                ],
            )?;
            Ok(changed)
        })
    }
}

fn require_one_row(changed: usize, id: AgentId) -> Result<(), PersistenceError> {
    if changed == 0 {
        return Err(PersistenceError::NotFound {
            what: format!("agent {id}"),
        });
    }
    Ok(())
}
