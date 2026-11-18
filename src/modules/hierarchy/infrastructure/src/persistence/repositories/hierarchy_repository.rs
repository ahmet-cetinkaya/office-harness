// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, PoisonError};

use hierarchy_application::common::errors::HierarchyApplicationError;
use hierarchy_application::common::interfaces::{HierarchyStore, TreeExclusion, TreeSnapshot};
use hierarchy_domain::Team;
use hierarchy_domain::entities::TeamId;
use hierarchy_domain::value_objects::{Role, TeamName};
use kernel::AgentId;
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::services::agent_places::AgentPlaces;

/// SQLite storage of the hierarchy: the team tables this module owns, and
/// the place columns of the agent module, written through `AgentPlaces`. The
/// connection is the handle of the one database the host app opened. One
/// call holds the connection lock for its whole closure, so a team write and
/// its member writes cannot interleave with another caller.
pub struct HierarchyRepository {
    connection: Arc<Mutex<Connection>>,
    /// Held across a count and the writes it feeds, so a new team cannot
    /// join the tree between the two. Taken before the connection lock.
    tree_writes: Mutex<()>,
    places: AgentPlaces,
}

impl HierarchyRepository {
    #[must_use]
    pub fn new(connection: Arc<Mutex<Connection>>, places: AgentPlaces) -> Self {
        Self {
            connection,
            tree_writes: Mutex::new(()),
            places,
        }
    }

    /// Runs `work` while holding the connection lock.
    fn locked<T>(
        &self,
        work: impl FnOnce(&mut Connection) -> Result<T, String>,
    ) -> Result<T, HierarchyApplicationError> {
        // A panic in another repository call leaves SQLite itself consistent,
        // so a poisoned lock is safe to reuse.
        let mut connection = self
            .connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        work(&mut connection).map_err(HierarchyApplicationError::Storage)
    }

    fn read_team(row: &Row<'_>) -> rusqlite::Result<Team> {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let project_id: Option<String> = row.get(2)?;
        let lead_agent_id: String = row.get(3)?;
        Ok(Team::stored(
            TeamId::from_uuid_text(&id).ok_or(rusqlite::Error::InvalidQuery)?,
            TeamName::parse(&name).map_err(|_| rusqlite::Error::InvalidQuery)?,
            project_id
                .map(|text| {
                    kernel::ProjectId::from_uuid_text(&text).ok_or(rusqlite::Error::InvalidQuery)
                })
                .transpose()?,
            AgentId::from_uuid_text(&lead_agent_id).ok_or(rusqlite::Error::InvalidQuery)?,
        ))
    }
}

impl HierarchyStore for HierarchyRepository {
    fn snapshot(&self) -> Result<TreeSnapshot, HierarchyApplicationError> {
        let places = &self.places;
        self.locked(|connection| {
            let rows = places.read_tree(connection)?;
            let teams = connection
                .prepare("SELECT id, name, project_id, lead_agent_id FROM hierarchy_teams ORDER BY rowid")
                .and_then(|mut statement| {
                    statement
                        .query_map([], Self::read_team)
                        .and_then(std::iter::Iterator::collect)
                })
                .map_err(|error| error.to_string())?;
            Ok(TreeSnapshot { rows, teams })
        })
    }

    fn tree_exclusion(&self) -> TreeExclusion<'_> {
        TreeExclusion::new(&self.tree_writes)
    }

    fn team_name_taken(&self, key: &str) -> Result<bool, HierarchyApplicationError> {
        self.locked(|connection| {
            connection
                .query_row(
                    "SELECT 1 FROM hierarchy_teams WHERE name_key = ?1",
                    params![key],
                    |_| Ok(()),
                )
                .optional()
                .map(|found| found.is_some())
                .map_err(|error| error.to_string())
        })
    }

    fn save_team_and_place_lead(
        &self,
        team: &Team,
        parent_agent_id: Option<AgentId>,
    ) -> Result<(), HierarchyApplicationError> {
        let places = &self.places;
        // The exclusion joins the new placement to the count a subtree
        // action took; the connection lock is always the inner one.
        let _exclusion = self.tree_exclusion();
        self.locked(|connection| {
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
                .map_err(|error| error.to_string())?;
            connection
                .execute(
                    "INSERT INTO hierarchy_team_members (agent_id, team_id, joined_at) \
                     VALUES (?1, ?2, ?3)",
                    params![
                        team.lead_agent_id().as_uuid_text(),
                        team.id().as_uuid_text(),
                        unix_now()
                    ],
                )
                .map_err(|error| error.to_string())?;
            places.write_place(
                connection,
                team.lead_agent_id(),
                Some(Role::Lead),
                parent_agent_id,
                Some(team.id()),
            )
        })
    }

    fn delete_team_and_unplace(&self, team_id: TeamId) -> Result<(), HierarchyApplicationError> {
        let places = &self.places;
        let _exclusion = self.tree_exclusion();
        self.locked(|connection| {
            let id = team_id.as_uuid_text();
            let members: Vec<String> = connection
                .prepare("SELECT agent_id FROM hierarchy_team_members WHERE team_id = ?1")
                .and_then(|mut statement| {
                    statement
                        .query_map(params![id], |row| row.get(0))
                        .and_then(|ids| ids.collect())
                })
                .map_err(|error| error.to_string())?;
            for member in &members {
                let agent_id = AgentId::from_uuid_text(member)
                    .ok_or_else(|| format!("agent id {member} is not a uuid"))?;
                places.write_place(connection, agent_id, None, None, None)?;
            }
            connection
                .execute(
                    "DELETE FROM hierarchy_team_members WHERE team_id = ?1",
                    params![id],
                )
                .map_err(|error| error.to_string())?;
            connection
                .execute("DELETE FROM hierarchy_teams WHERE id = ?1", params![id])
                .map(|_| ())
                .map_err(|error| error.to_string())
        })
    }

    fn place_agent(
        &self,
        agent_id: AgentId,
        role: Option<Role>,
        parent_agent_id: Option<AgentId>,
        team_id: Option<TeamId>,
    ) -> Result<(), HierarchyApplicationError> {
        let places = &self.places;
        // No exclusion of its own: the caller holds it across the check and
        // this write (the create-role handler's contract), and a second
        // take of the same mutex would never return.
        self.locked(|connection| {
            places.write_place(connection, agent_id, role, parent_agent_id, team_id)?;
            if let Some(team) = team_id {
                connection
                    .execute(
                        "INSERT OR REPLACE INTO hierarchy_team_members \
                         (agent_id, team_id, joined_at) VALUES (?1, ?2, ?3)",
                        params![agent_id.as_uuid_text(), team.as_uuid_text(), unix_now()],
                    )
                    .map(|_| ())
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        })
    }

    fn team_name_of(&self, team_id: TeamId) -> Result<Option<TeamName>, HierarchyApplicationError> {
        self.locked(|connection| {
            connection
                .query_row(
                    "SELECT name FROM hierarchy_teams WHERE id = ?1",
                    params![team_id.as_uuid_text()],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|error| error.to_string())
                .and_then(|name| {
                    name.map(|text| TeamName::parse(&text).map_err(|error| error.to_string()))
                        .transpose()
                })
        })
    }
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs().try_into().unwrap_or_default())
}
