// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs::OpenOptions;
use std::path::Path;

use private_fs::open_private_file;
use rusqlite::Connection;

use super::StartupError;

const CONNECTION_SETTINGS: &str = "
    PRAGMA journal_mode = WAL;
    PRAGMA foreign_keys = ON;
    PRAGMA busy_timeout = 5000;
    PRAGMA secure_delete = ON;
";

/// Opens `office.db`, creating it owner-only first, in WAL mode with foreign
/// keys enforced.
///
/// # Errors
/// `DataFolder` when the file cannot be created, `Database` when SQLite fails.
pub fn open_database(path: &Path) -> Result<Connection, StartupError> {
    open_private_file(path, OpenOptions::new().write(true))?;
    let connection = Connection::open(path)?;
    connection.execute_batch(CONNECTION_SETTINGS)?;
    Ok(connection)
}
