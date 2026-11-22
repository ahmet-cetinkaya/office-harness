// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use agent_domain::entities::Agent;
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{AgentCommand, AgentWorktree, DisplayName, TaskSource, TaskText};
use kernel::{AgentId, Timestamp};
use rusqlite::{OptionalExtension, Row, params};

use crate::persistence::{PersistenceError, SharedConnection};

const SELECT_AGENT: &str = "SELECT id, name, command, working_folder, project_id, machine_id, \
                            status, created_at, task_text, task_set_at, task_source, \
                            worktree_repository, worktree_base, worktree_branch, \
                            worktree_folder FROM agent_agents";

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
    task: StoredTask,
    worktree: StoredWorktree,
}

/// The three task columns as stored; they are all set or all NULL.
struct StoredTask {
    text: Option<String>,
    set_at: Option<i64>,
    source: Option<String>,
}

/// The four worktree columns as stored; they are all set or all NULL.
struct StoredWorktree {
    repository: Option<String>,
    base: Option<String>,
    branch: Option<String>,
    folder: Option<String>,
}

impl StoredWorktree {
    /// A row with only some of the columns set reads as no worktree, as a
    /// partly written task does: the agent still lists and starts.
    fn into_worktree(self) -> Option<AgentWorktree> {
        let (Some(repository), Some(base), Some(branch), Some(folder)) =
            (self.repository, self.base, self.branch, self.folder)
        else {
            return None;
        };
        Some(AgentWorktree {
            repository: PathBuf::from(repository),
            base,
            branch,
            folder: PathBuf::from(folder),
        })
    }
}

impl StoredTask {
    /// A row that the domain rejects (too long, or a source this version does
    /// not know) reads as no task: a task line is not worth failing the read
    /// of every agent, and the next change to the task overwrites the row.
    fn into_task(self) -> Option<TaskText> {
        let (Some(text), Some(set_at), Some(source)) = (self.text, self.set_at, self.source) else {
            return None;
        };
        let source = source.parse::<TaskSource>().ok()?;
        TaskText::from_input(&text, Timestamp::from_unix_millis(set_at), source)
            .ok()
            .flatten()
    }
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
            task: StoredTask {
                text: row.get(8)?,
                set_at: row.get(9)?,
                source: row.get(10)?,
            },
            worktree: StoredWorktree {
                repository: row.get(11)?,
                base: row.get(12)?,
                branch: row.get(13)?,
                folder: row.get(14)?,
            },
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
        agent.task = self.task.into_task();
        agent.worktree = self.worktree.into_worktree();
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
        let worktree = agent.worktree.as_ref();
        self.connection.with(|connection| {
            connection
                .execute(
                    "INSERT INTO agent_agents (id, name, name_key, command, working_folder, \
                     project_id, machine_id, status, created_at, worktree_repository, \
                     worktree_base, worktree_branch, worktree_folder) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
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
                        worktree.map(|worktree| worktree.repository.to_string_lossy()),
                        worktree.map(|worktree| worktree.base.as_str()),
                        worktree.map(|worktree| worktree.branch.as_str()),
                        worktree.map(|worktree| worktree.folder.to_string_lossy()),
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

    /// Stores the task text, or clears all three columns for `None`.
    ///
    /// # Errors
    /// `NotFound` when no agent has the id.
    pub fn update_task(
        &self,
        id: AgentId,
        task: Option<&TaskText>,
    ) -> Result<(), PersistenceError> {
        self.connection.with(|connection| {
            let changed = connection.execute(
                "UPDATE agent_agents SET task_text = ?2, task_set_at = ?3, task_source = ?4 \
                 WHERE id = ?1",
                params![
                    id.to_string(),
                    task.map(TaskText::text),
                    task.map(|task| task.set_at().as_unix_millis()),
                    task.map(|task| task.source().as_str()),
                ],
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
