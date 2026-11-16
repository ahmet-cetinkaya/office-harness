// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs;
use std::io;
use std::path::PathBuf;

use directories::ProjectDirs;
use thiserror::Error;
use wire::SERVER_FILE_NAME;

use super::server_file::ServerFile;

// The same names as the server uses to pick its default data folder.
const QUALIFIER: &str = "dev";
const ORGANIZATION: &str = "office-harness";
const APPLICATION: &str = "office-harness";
const TOKEN_FILE_NAME: &str = "token";

#[derive(Debug, Error)]
pub enum DataFolderError {
    #[error("the user's data folder cannot be determined: no home folder")]
    NoHomeFolder,
}

#[derive(Debug, Error)]
pub enum ReadError {
    #[error("the file cannot be read: {0}")]
    Io(#[from] io::Error),
    #[error("server.json is not valid: {0}")]
    Invalid(#[from] serde_json::Error),
    #[error("the token file is empty")]
    EmptyToken,
}

/// The folder where the server keeps `server.json` and the token.
#[derive(Debug, Clone)]
pub struct DataFolder {
    path: PathBuf,
}

impl DataFolder {
    pub fn locate() -> Result<Self, DataFolderError> {
        ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
            .map(|directories| Self {
                path: directories.data_local_dir().to_path_buf(),
            })
            .ok_or(DataFolderError::NoHomeFolder)
    }

    pub fn read_server_file(&self) -> Result<ServerFile, ReadError> {
        let text = fs::read_to_string(self.path.join(SERVER_FILE_NAME))?;
        Ok(ServerFile::parse(&text)?)
    }

    /// The token is read at each attempt, so a token that the user fixed or
    /// that the server replaced is picked up without a restart of the app.
    pub fn read_token(&self) -> Result<String, ReadError> {
        let text = fs::read_to_string(self.path.join(TOKEN_FILE_NAME))?;
        let token = text.trim();
        if token.is_empty() {
            return Err(ReadError::EmptyToken);
        }
        Ok(token.to_owned())
    }

    #[cfg(test)]
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }
}

#[cfg(test)]
#[path = "data_folder.tests.rs"]
mod tests;
