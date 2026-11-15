// SPDX-License-Identifier: AGPL-3.0-or-later

use rusqlite::{Connection, params};

use super::StartupError;

/// Brings one module's tables forward: script `n` is version `n`, and the
/// module's ledger table records the versions that ran. Each script runs in a
/// transaction with its ledger row, so a failure leaves the previous version.
/// Returns how many scripts ran; a database that is current runs none.
///
/// # Errors
/// `Migration` when a script fails, `DatabaseTooNew` when the ledger is ahead
/// of `scripts`, and `InvalidLedgerName` for a name that is not plain
/// lower-case letters and underscores.
pub fn run_migrations(
    connection: &mut Connection,
    ledger_table: &str,
    scripts: &[&str],
) -> Result<usize, StartupError> {
    if !is_plain_table_name(ledger_table) {
        return Err(StartupError::InvalidLedgerName(ledger_table.to_owned()));
    }
    connection.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {ledger_table} (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL DEFAULT (unixepoch('subsec') * 1000)
        )"
    ))?;
    let applied = current_version(connection, ledger_table)?;
    let supported = version_of(scripts.len());
    if applied > supported {
        return Err(StartupError::DatabaseTooNew {
            ledger: ledger_table.to_owned(),
            found: applied,
            supported,
        });
    }
    let already_applied = usize::try_from(applied).unwrap_or(scripts.len());
    for (index, script) in scripts.iter().enumerate().skip(already_applied) {
        let version = version_of(index + 1);
        apply_script(connection, ledger_table, version, script).map_err(|source| {
            StartupError::Migration {
                ledger: ledger_table.to_owned(),
                version,
                source,
            }
        })?;
    }
    Ok(scripts.len() - already_applied)
}

fn version_of(script_count: usize) -> i64 {
    i64::try_from(script_count).unwrap_or(i64::MAX)
}

fn is_plain_table_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_')
}

fn current_version(connection: &Connection, ledger_table: &str) -> rusqlite::Result<i64> {
    connection.query_row(
        &format!("SELECT COALESCE(MAX(version), 0) FROM {ledger_table}"),
        [],
        |row| row.get(0),
    )
}

fn apply_script(
    connection: &mut Connection,
    ledger_table: &str,
    version: i64,
    script: &str,
) -> rusqlite::Result<()> {
    let transaction = connection.transaction()?;
    transaction.execute_batch(script)?;
    transaction.execute(
        &format!("INSERT INTO {ledger_table} (version) VALUES (?1)"),
        params![version],
    )?;
    transaction.commit()
}
