// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ffi::OsString;
use std::path::PathBuf;

use directories::ProjectDirs;

use super::ConfigError;

const DATA_FOLDER_FLAG: &str = "--data-folder";
const QUALIFIER: &str = "dev";
const ORGANIZATION: &str = "office-harness";
const APPLICATION: &str = "office-harness";
const DATABASE_FILE: &str = "office.db";
const LOCK_FILE: &str = "server.lock";
const LOG_FILE: &str = "server.log";
const TOKEN_FILE: &str = "token";

/// Where the server keeps its state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub data_folder: PathBuf,
}

impl Config {
    /// Reads the command-line arguments (without the program name). The data
    /// folder is the user's local application data folder unless
    /// `--data-folder <path>` names another one.
    ///
    /// # Errors
    /// An unknown argument, a flag without a value, or no home folder.
    pub fn load(arguments: impl IntoIterator<Item = OsString>) -> Result<Self, ConfigError> {
        let mut arguments = arguments.into_iter();
        let mut data_folder = None;
        while let Some(argument) = arguments.next() {
            if argument != DATA_FOLDER_FLAG {
                return Err(ConfigError::UnknownArgument(
                    argument.to_string_lossy().into_owned(),
                ));
            }
            let value = arguments
                .next()
                .ok_or(ConfigError::MissingValue(DATA_FOLDER_FLAG))?;
            data_folder = Some(PathBuf::from(value));
        }
        Ok(Self {
            data_folder: data_folder.map_or_else(default_data_folder, Ok)?,
        })
    }

    #[must_use]
    pub fn database_path(&self) -> PathBuf {
        self.data_folder.join(DATABASE_FILE)
    }

    #[must_use]
    pub fn lock_path(&self) -> PathBuf {
        self.data_folder.join(LOCK_FILE)
    }

    #[must_use]
    pub fn token_path(&self) -> PathBuf {
        self.data_folder.join(TOKEN_FILE)
    }

    #[must_use]
    pub fn server_file_path(&self) -> PathBuf {
        self.data_folder.join(wire::SERVER_FILE_NAME)
    }

    #[must_use]
    pub fn log_path(&self) -> PathBuf {
        self.data_folder.join(LOG_FILE)
    }
}

fn default_data_folder() -> Result<PathBuf, ConfigError> {
    ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
        .map(|directories| directories.data_local_dir().to_path_buf())
        .ok_or(ConfigError::NoHomeFolder)
}

#[cfg(test)]
#[path = "server_config.tests.rs"]
mod tests;
