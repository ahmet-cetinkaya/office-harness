// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io;
use std::path::PathBuf;

use thiserror::Error;

use super::LoginItemError;
use crate::config::ConfigError;

const FAILURE_EXIT_CODE: u8 = 1;
const USAGE_EXIT_CODE: u8 = 2;

#[derive(Debug, Error)]
pub enum CliError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// The login item always serves the default data folder, so the
    /// subcommands take no folder of their own.
    #[error("unknown argument for this subcommand: {0}")]
    UnknownArgument(String),
    #[error("the running program cannot be found: {0}")]
    NoRunningProgram(io::Error),
    #[error("cannot install or remove {}: {source}", .path.display())]
    Files { path: PathBuf, source: io::Error },
    #[error(transparent)]
    LoginItem(#[from] LoginItemError),
}

impl CliError {
    /// The process exit code that reports this error. A bad argument is
    /// different from a failure of the operating system.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Config(_) | Self::UnknownArgument(_) => USAGE_EXIT_CODE,
            _ => FAILURE_EXIT_CODE,
        }
    }
}
