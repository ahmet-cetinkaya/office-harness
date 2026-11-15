// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io;
use std::path::PathBuf;

use agent_infrastructure::persistence::PersistenceError;
use thiserror::Error;

use crate::config::ConfigError;

#[derive(Debug, Error)]
pub enum StartupError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("another server already runs for the data folder {}", .0.display())]
    AlreadyRunning(PathBuf),
    #[error("cannot prepare the data folder or its files: {0}")]
    DataFolder(#[from] io::Error),
    #[error("database failure: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("{ledger} migration failed at version {version}: {source}")]
    Migration {
        ledger: String,
        version: i64,
        source: rusqlite::Error,
    },
    #[error(
        "the database ledger {ledger} is at version {found}, newer than the {supported} that this server knows"
    )]
    DatabaseTooNew {
        ledger: String,
        found: i64,
        supported: i64,
    },
    #[error("{0} is not a valid ledger table name")]
    InvalidLedgerName(String),
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
}
