// SPDX-License-Identifier: AGPL-3.0-or-later

use hierarchy_domain::TreeRow;
use hierarchy_domain::entities::TeamId;
use hierarchy_domain::value_objects::Role;
use kernel::AgentId;
use rusqlite::{Connection, OptionalExtension, Row, params};

/// Reads and writes the place columns of the agent module: role,
/// `parent_agent_id`, and `team_id`. The columns are added by the agent module's
/// migration 0004; this service is the one writer.
pub struct AgentPlaces;

impl AgentPlaces {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Reads the whole tree in one SELECT.
    ///
    /// # Errors
    /// A SQLite error, or a row whose ids or role are not valid.
    pub fn read_tree(&self, connection: &Connection) -> Result<Vec<TreeRow>, String> {
        connection
            .prepare(
                "SELECT id, role, parent_agent_id, team_id FROM agent_agents \
                 WHERE role IS NOT NULL",
            )
            .map_err(|error| error.to_string())
            .and_then(|mut statement| {
                let rows = statement
                    .query_map([], read_row)
                    .map_err(|error| error.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| error.to_string())?;
                Ok(rows)
            })
    }

    /// Writes one agent's place columns.
    ///
    /// # Errors
    /// A SQLite error.
    pub fn write_place(
        &self,
        connection: &Connection,
        agent_id: AgentId,
        role: Option<Role>,
        parent_agent_id: Option<AgentId>,
        team_id: Option<TeamId>,
    ) -> Result<(), String> {
        connection
            .execute(
                "UPDATE agent_agents SET role = ?1, parent_agent_id = ?2, team_id = ?3 \
                 WHERE id = ?4",
                params![
                    role.map(hierarchy_domain::Role::as_str),
                    parent_agent_id.map(|id| id.as_uuid_text()),
                    team_id.map(|id| id.as_uuid_text()),
                    agent_id.as_uuid_text()
                ],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Whether the place columns exist yet (migration 0004 has run).
    ///
    /// # Errors
    /// A SQLite error.
    pub fn columns_exist(&self, connection: &Connection) -> Result<bool, String> {
        connection
            .query_row(
                "SELECT 1 FROM pragma_table_info('agent_agents') WHERE name = 'role'",
                [],
                |_| Ok(()),
            )
            .optional()
            .map(|found| found.is_some())
            .map_err(|error| error.to_string())
    }
}

fn read_row(row: &Row<'_>) -> rusqlite::Result<TreeRow> {
    let id: String = row.get(0)?;
    let role: Option<String> = row.get(1)?;
    let parent: Option<String> = row.get(2)?;
    let team: Option<String> = row.get(3)?;
    Ok(TreeRow {
        agent_id: AgentId::from_uuid_text(&id).ok_or(rusqlite::Error::InvalidQuery)?,
        role: role.as_deref().and_then(Role::from_wire),
        parent_agent_id: parent
            .map(|text| AgentId::from_uuid_text(&text).ok_or(rusqlite::Error::InvalidQuery))
            .transpose()?,
        team_id: team
            .map(|text| TeamId::from_uuid_text(&text).ok_or(rusqlite::Error::InvalidQuery))
            .transpose()?,
    })
}

impl Default for AgentPlaces {
    fn default() -> Self {
        Self::new()
    }
}
