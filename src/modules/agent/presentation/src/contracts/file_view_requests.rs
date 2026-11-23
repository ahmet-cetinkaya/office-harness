// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Map, Value};

use super::Fields;

/// `agent.files.list`: `path` is `""` for the root (0016 contract).
#[derive(Debug)]
pub struct FilesListRequest {
    pub agent_id: String,
    pub path: String,
}

impl FilesListRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        Ok(Self {
            agent_id: fields.required_text("agent_id")?,
            path: fields.required_text("path")?,
        })
    }
}

/// `agent.files.read`: a path and whether a secret-looking file is shown.
#[derive(Debug)]
pub struct FilesReadRequest {
    pub agent_id: String,
    pub path: String,
    pub reveal: bool,
}

impl FilesReadRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        Ok(Self {
            agent_id: fields.required_text("agent_id")?,
            path: fields.required_text("path")?,
            reveal: fields.optional_bool("reveal")?.unwrap_or(false),
        })
    }
}

/// `agent.files.diff`.
#[derive(Debug)]
pub struct FilesDiffRequest {
    pub agent_id: String,
    pub path: String,
    pub old_path: Option<String>,
    pub commit: Option<String>,
    pub reveal: bool,
}

impl FilesDiffRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        Ok(Self {
            agent_id: fields.required_text("agent_id")?,
            path: fields.required_text("path")?,
            old_path: fields.optional_text("old_path")?,
            commit: fields.optional_text("commit")?,
            reveal: fields.optional_bool("reveal")?.unwrap_or(false),
        })
    }
}

/// `agent.files.history`: how many commits to skip, 0 by default.
#[derive(Debug)]
pub struct FilesHistoryRequest {
    pub agent_id: String,
    pub skip: u32,
}

impl FilesHistoryRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        Ok(Self {
            agent_id: fields.required_text("agent_id")?,
            skip: fields.optional_u32("skip")?.unwrap_or(0),
        })
    }
}

/// `agent.files.commit`.
#[derive(Debug)]
pub struct FilesCommitRequest {
    pub agent_id: String,
    pub commit: String,
}

impl FilesCommitRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        Ok(Self {
            agent_id: fields.required_text("agent_id")?,
            commit: fields.required_text("commit")?,
        })
    }
}
