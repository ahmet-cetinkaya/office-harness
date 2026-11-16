// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use hierarchy_application::common::errors::HierarchyApplicationError;
use hierarchy_application::common::interfaces::{HierarchyStore, TreeSnapshot};
use hierarchy_domain::entities::TeamId;
use hierarchy_domain::value_objects::{Role, TeamName};
use hierarchy_domain::{Team, TreeRow};
use kernel::AgentId;
use rusqlite::{Connection, Row, params};

use crate::services::agent_places::AgentPlaces;

/// SQLite storage of the hierarchy: the team tables this module owns, and
/// the place columns of the agent module, written through `AgentPlaces`.
pub struct HierarchyRepository {
    connection: Arc<Connection>,
    places: AgentPlaces,
}

impl HierarchyRepository {
    #[must_use]
    pub fn new(connection: Arc<Connection>, places: AgentPlaces) -> Self {
        Self { connection, places }
    }

    fn read_team(row: &Row<'_>) -> rusqlite::Result<Team> {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let project_id: Option<String> = row.get(2)?;
        let lead_agent_id: String = row.get(3)?;
        let team_id = TeamId::from_uuid_text(&id).ok_or(rusqlite::Error::InvalidQuery)?;
        let lead = AgentId::from_uuid_text(&lead_agent_id).ok_or(rusqlite::Error::InvalidQuery)?;
        let display_name = TeamName::parse(&name).map_err(|_| rusqlite::Error::InvalidQuery)?;
        Ok(Team::stored(
            team_id,
            display_name,
            project_id
                .map(|text| {
                    agent_domain::value_objects::ProjectId::from_uuid_text(&text)
                        .ok_or(rusqlite::Error::InvalidQuery)
                })
                .transpose()?,
            lead,
        ))
    }

    /// Runs `work` inside one transaction, so a team write and its member
    /// writes go together or not at all.
    fn in_transaction<T>(
        &self,
        work: impl FnOnce(&Connection) -> Result<T, HierarchyApplicationError>,
    ) -> Result<T, HierarchyApplicationError> {
        self.connection
            .unchecked_transaction()
            .map_err(|error| HierarchyApplicationError::Storage(error.to_string()))
            .and_then(|transaction| {
                let result = work(&transaction)?;
                transaction
                    .commit()
                    .map_err(|error| HierarchyApplicationError::Storage(error.to_string()))?;
                Ok(result)
            })
    }
}

impl HierarchyStore for HierarchyRepository {
    fn snapshot(&self) -> Result<TreeSnapshot, HierarchyApplicationError> {
        let rows = self
            .places
            .read_tree(self.connection.as_ref())
            .map_err(|error| HierarchyApplicationError::Storage(error.to_string()))?;
        let teams = self
            .connection
            .prepare("SELECT id, name, project_id, lead_agent_id FROM hierarchy_teams ORDER BY rowid")
            .and_then(|mut statement| {
                statement
                    .query_map([], Self::read_team)
                    .and_then(|teams| teams.collect())
            })
            .map_err(|error| HierarchyApplicationError::Storage(error.to_string()))?;
        Ok(TreeSnapshot { rows, teams })
    }

    fn team_name_taken(&self, key: &str) -> Result<bool, HierarchyApplicationError> {
        self.connection
            .query_row(
                "SELECT 1 FROM hierarchy_teams WHERE name_key = ?1",
                params![key],
                |_| Ok(()),
            )
            .optional()
            .map(|found| found.is_some())
            .map_err(|error| HierarchyApplicationError::Storage(error.to_string()))
    }

    fn save_team_and_place_lead(
        &self,
        team: &Team,
        parent_agent_id: Option<AgentId>,
    ) -> Result<(), HierarchyApplicationError> {
        self.in_transaction(|connection| {
            connection
                .execute(
                    "INSERT INTO hierarchy_teams (id, name, name_key, project_id, lead_agent_id) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        team.id().as_uuid_text(),
                        team.name().display(),
                        team.name().key(),
                        team.project_id().map(|id| id.as_uuid_text()),
                        team.lead_agent_id().as_uuid_text()
                    ],
                )
                .map_err(|error| HierarchyApplicationError::Storage(error.to_string()))?;
            connection
                .execute(
                    "INSERT INTO hierarchy_team_members (agent_id, team_id, joined_at) \
                     VALUES (?1, ?2, ?3)",
                    params![
                        team.lead_agent_id().as_uuid_text(),
                        team.id().as_uuid_text(),
                        now_unix()
                    ],
                )
                .map_err(|error| HierarchyApplicationError::Storage(error.to_string()))?;
            self.places
                .write_place(
                    connection,
                    team.lead_agent_id(),
                    Some(Role::Lead),
                    parent_agent_id,
                    Some(team.id()),
                )
                .map_err(|error| HierarchyApplicationError::Storage(error.to_string()))
        })
    }

    fn delete_team_and_unplace(&self, team_id: TeamId) -> Result<(), HierarchyApplicationError> {
        self.in_transaction(|connection| {
            let id = team_id.as_uuid_text();
            let members: Vec<String> = connection
                .prepare("SELECT agent_id FROM hierarchy_team_members WHERE team_id = ?1")
                .and_then(|mut statement| {
                    statement.query_map(params![id], |row| row.get(0)).and_then(|ids| ids.collect())
                })
                .map_err(|error| HierarchyApplicationError::Storage(error.to_string()))?;
            for member in &members {
                self.places
                    .write_place(connection, member_id(member)?, None, None, None)
                    .map_err(|error| HierarchyApplicationError::Storage(error.to_string()))?;
            }
            connection
                .execute("DELETE FROM hierarchy_team_members WHERE team_id = ?1", params![id])
                .and_then(|()| {
                    connection.execute("DELETE FROM hierarchy_teams WHERE id = ?1", params![id])
                })
                .map(|_| ())
                .map_err(|error| HierarchyApplicationError::Storage(error.to_string()))
        })
    }

    fn place_agent(
        &self,
        agent_id: AgentId,
        role: Option<Role>,
        parent_agent_id: Option<AgentId>,
        team_id: Option<TeamId>,
    ) -> Result<(), HierarchyApplicationError> {
        self.connection
            .unchecked_transaction()
            .map_err(|error| HierarchyApplicationError::Storage(error.to_string()))
            .and_then(|transaction| {
                self.places
                    .write_place(&transaction, agent_id, role, parent_agent_id, team_id)
                    .map_err(|error| HierarchyApplicationError::Storage(error.to_string()))?;
                if let Some(team) = team_id {
                    transaction
                        .execute(
                            "INSERT OR REPLACE INTO hierarchy_team_members \
                             (agent_id, team_id, joined_at) VALUES (?1, ?2, ?3)",
                            params![agent_id.as_uuid_text(), team.as_uuid_text(), now_unix()],
                        )
                        .map(|_| ())
                        .map_err(|error| HierarchyApplicationError::Storage(error.to_string()))?;
                }
                transaction
                    .commit()
                    .map_err(|error| HierarchyApplicationError::Storage(error.to_string()))
            })
    }

    fn team_name_of(&self, team_id: TeamId) -> Result<Option<TeamName>, HierarchyApplicationError> {
        self.connection
            .query_row(
                "SELECT name FROM hierarchy_teams WHERE id = ?1",
                params![team_id.as_uuid_text()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| HierarchyApplicationError::Storage(error.to_string()))
            .and_then(|name| {
                name.map(|text| TeamName::parse(&text).map_err(hierarchy_invalid))
                    .transpose()
            })
    }
}

fn hierarchy_invalid(error: hierarchy_domain::HierarchyError) -> HierarchyApplicationError {
    HierarchyApplicationError::Invalid(error)
}

fn member_id(text: &str) -> Result<AgentId, HierarchyApplicationError> {
    AgentId::from_uuid_text(text).ok_or_else(|| {
        HierarchyApplicationError::Storage(format!("agent id {text} is not a uuid"))
    })
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or_default()
}
