// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use agent_domain::entities::Project;
use agent_domain::value_objects::{DisplayName, ProjectId};
use rusqlite::{OptionalExtension, Row, Transaction, params};

use crate::persistence::{PersistenceError, SharedConnection};

const SELECT_PROJECT: &str = "SELECT id, name, folder FROM agent_projects";

pub struct ProjectRepository {
    connection: SharedConnection,
}

struct ProjectRow {
    id: String,
    name: String,
    folder: String,
}

impl ProjectRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            folder: row.get(2)?,
        })
    }

    fn into_project(self) -> Result<Project, PersistenceError> {
        Ok(Project::new(
            self.id.parse()?,
            DisplayName::parse("project name", &self.name)?,
            PathBuf::from(self.folder),
        )?)
    }
}

impl ProjectRepository {
    #[must_use]
    pub fn new(connection: SharedConnection) -> Self {
        Self { connection }
    }

    /// # Errors
    /// `Duplicate` when another project has the name, ignoring case.
    pub fn insert(&self, project: &Project) -> Result<(), PersistenceError> {
        self.connection.with(|connection| {
            connection
                .execute(
                    "INSERT INTO agent_projects (id, name, name_key, folder) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        project.id.to_string(),
                        project.name.as_str(),
                        project.name.uniqueness_key(),
                        project.folder.to_string_lossy(),
                    ],
                )
                .map_err(|error| {
                    PersistenceError::from_insert(error, format!("project {}", project.name))
                })?;
            Ok(())
        })
    }

    /// # Errors
    /// A storage failure or a stored row that the domain rejects.
    pub fn find(&self, id: ProjectId) -> Result<Option<Project>, PersistenceError> {
        self.connection.with(|connection| {
            let row = connection
                .query_row(
                    &format!("{SELECT_PROJECT} WHERE id = ?1"),
                    [id.to_string()],
                    ProjectRow::read,
                )
                .optional()?;
            row.map(ProjectRow::into_project).transpose()
        })
    }

    /// # Errors
    /// A storage failure or a stored row that the domain rejects.
    pub fn list(&self) -> Result<Vec<Project>, PersistenceError> {
        self.connection.with(|connection| {
            let mut statement =
                connection.prepare(&format!("{SELECT_PROJECT} ORDER BY name_key"))?;
            let rows = statement
                .query_map([], ProjectRow::read)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows.into_iter().map(ProjectRow::into_project).collect()
        })
    }

    /// Deletes the project unless agents still use it.
    ///
    /// # Errors
    /// `ProjectInUse` naming those agents, or `NotFound`.
    pub fn delete(&self, id: ProjectId) -> Result<(), PersistenceError> {
        self.connection.with(|connection| {
            let transaction = connection.transaction()?;
            let name = project_name(&transaction, id)?;
            let agents = agent_names_using(&transaction, id)?;
            if !agents.is_empty() {
                return Err(PersistenceError::ProjectInUse {
                    project: name,
                    agents,
                });
            }
            transaction.execute("DELETE FROM agent_projects WHERE id = ?1", [id.to_string()])?;
            Ok(transaction.commit()?)
        })
    }
}

fn project_name(transaction: &Transaction<'_>, id: ProjectId) -> Result<String, PersistenceError> {
    transaction
        .query_row(
            "SELECT name FROM agent_projects WHERE id = ?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| PersistenceError::NotFound {
            what: format!("project {id}"),
        })
}

fn agent_names_using(
    transaction: &Transaction<'_>,
    id: ProjectId,
) -> Result<Vec<String>, PersistenceError> {
    let mut statement = transaction
        .prepare("SELECT name FROM agent_agents WHERE project_id = ?1 ORDER BY name_key")?;
    let names = statement
        .query_map([id.to_string()], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    Ok(names)
}
