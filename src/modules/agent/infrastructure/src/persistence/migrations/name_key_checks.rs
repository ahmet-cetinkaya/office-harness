// SPDX-License-Identifier: AGPL-3.0-or-later

use rusqlite::Connection;

/// One row of a colliding group: (id, name, `name_key`).
type NameKeyRow = (String, String, String);

/// The migration version whose unique NOCASE index a group of `name_key`
/// values that differ only by ASCII case would break.
pub const NAME_KEY_INDEX_VERSION: i64 = 4;

/// The rows whose `name_key` differ only by ASCII case. The comparison folds
/// ASCII letters only, exactly like the index that the version builds.
const COLLIDING_ROWS_SQL: &str = "
    SELECT id, name, name_key
    FROM {table}
    WHERE name_key COLLATE NOCASE IN (
        SELECT name_key FROM {table}
        GROUP BY name_key COLLATE NOCASE HAVING COUNT(*) > 1
    )
    ORDER BY name_key COLLATE NOCASE, id";

/// Refuses the step to version 4 while rows hold `name_key` values that
/// differ only by ASCII case: the unique NOCASE indexes of that version would
/// reject them with a bare constraint error. A table that does not exist yet
/// (a fresh database) holds no rows, so it passes. The check only reads; the
/// ledger stays at its old version and every row is kept.
///
/// # Errors
/// The table, id, name, and `name_key` of every row in each colliding group,
/// and the fix: rename or delete all but one row of each group, then start
/// again.
pub fn check_name_key_ascii_groups(connection: &Connection) -> Result<(), String> {
    let groups = [
        table_group(connection, "agent_projects")?,
        table_group(connection, "agent_agents")?,
    ];
    if groups.iter().all(|(_, rows)| rows.is_empty()) {
        return Ok(());
    }
    Err(problem_text(&groups))
}

fn table_group<'a>(
    connection: &Connection,
    table: &'a str,
) -> Result<(&'a str, Vec<NameKeyRow>), String> {
    let rows = colliding_rows(connection, table)
        .map_err(|error| format!("cannot read the name keys of {table}: {error}"))?;
    Ok((table, rows))
}

/// The colliding rows of one table; a table that script 1 has not created yet
/// counts as empty. Any other failure (a damaged schema, a locked database)
/// surfaces instead of passing the check silently.
fn colliding_rows(connection: &Connection, table: &str) -> rusqlite::Result<Vec<NameKeyRow>> {
    if !table_exists(connection, table)? {
        return Ok(Vec::new());
    }
    let sql = COLLIDING_ROWS_SQL.replace("{table}", table);
    let mut statement = connection.prepare(&sql)?;
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect()
}

fn table_exists(connection: &Connection, table: &str) -> rusqlite::Result<bool> {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get::<_, i64>(0),
        )
        .map(|found| found > 0)
}

fn problem_text(groups: &[(&str, Vec<NameKeyRow>)]) -> String {
    let listed = groups
        .iter()
        .filter(|(_, rows)| !rows.is_empty())
        .map(|(table, rows)| describe_group(table, rows))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "the agent tables hold names that differ only by ASCII case, which the \
         unique indexes of migration {NAME_KEY_INDEX_VERSION} would refuse:\n{listed}\n\
         rename or delete all but one row of each group above, then start again"
    )
}

fn describe_group(table: &str, rows: &[NameKeyRow]) -> String {
    let listed = rows
        .iter()
        .map(|(id, name, name_key)| {
            format!("  - {table} id {id}, name \"{name}\", name_key \"{name_key}\"")
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("in {table}:\n{listed}")
}

#[cfg(test)]
#[path = "name_key_checks.tests.rs"]
mod tests;
