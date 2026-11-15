// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use crate::errors::AgentError;
use crate::value_objects::{DisplayName, ProjectId};

/// A named folder that agents work in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub id: ProjectId,
    pub name: DisplayName,
    pub folder: PathBuf,
}

impl Project {
    /// # Errors
    /// `RelativePath` when the folder is not absolute.
    pub fn new(id: ProjectId, name: DisplayName, folder: PathBuf) -> Result<Self, AgentError> {
        if !folder.is_absolute() {
            return Err(AgentError::RelativePath {
                field: "project folder",
            });
        }
        Ok(Self { id, name, folder })
    }
}
