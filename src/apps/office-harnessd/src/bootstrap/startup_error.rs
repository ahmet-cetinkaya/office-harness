// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io;
use std::path::PathBuf;

use access_application::common::errors::AccessError;
use agent_infrastructure::persistence::PersistenceError;
use terminal_domain::errors::TerminalError;
use thiserror::Error;

use crate::config::ConfigError;
use crate::server::ServerError;

/// The exit code of a server that finds another one running for the same data
/// folder, so that a launcher can tell it from a real failure.
pub const ALREADY_RUNNING_EXIT_CODE: u8 = 75;

const FAILURE_EXIT_CODE: u8 = 1;

#[derive(Debug, Error)]
pub enum StartupError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("another server already runs for the data folder {}", .0.display())]
    AlreadyRunning(PathBuf),
    #[error("cannot prepare the data folder or its files: {0}")]
    DataFolder(#[from] io::Error),
    #[error("cannot install the shutdown signal handler: {0}")]
    SignalHandler(io::Error),
    #[error("database failure: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("{ledger} migration failed at version {version}: {source}")]
    Migration {
        ledger: String,
        version: i64,
        source: rusqlite::Error,
    },
    #[error("{ledger} migration stopped before version {version}: {message}")]
    PreMigrationCheck {
        ledger: String,
        version: i64,
        message: String,
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
    #[error(transparent)]
    Access(#[from] AccessError),
    #[error("terminal setting: {0}")]
    Terminal(#[from] TerminalError),
    #[error(transparent)]
    Server(#[from] ServerError),
}

impl StartupError {
    /// The process exit code that reports this error.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::AlreadyRunning(_) => ALREADY_RUNNING_EXIT_CODE,
            _ => FAILURE_EXIT_CODE,
        }
    }
}
