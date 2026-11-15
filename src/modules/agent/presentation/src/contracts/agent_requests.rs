// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Map, Value};

use super::Fields;

/// `agent.create`. Office additions (`additions`) arrive with the additions
/// change; until then the field is not read.
#[derive(Debug)]
pub struct CreateAgentRequest {
    pub name: String,
    pub command: String,
    pub working_folder: Option<String>,
    pub project_id: Option<String>,
}

impl CreateAgentRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        Ok(Self {
            name: fields.required_text("name")?,
            command: fields.required_text("command")?,
            working_folder: fields.optional_text("working_folder")?,
            project_id: fields.optional_text("project_id")?,
        })
    }
}

/// `agent.delete`, `agent.start`, and `agent.stop`.
#[derive(Debug)]
pub struct AgentIdRequest {
    pub agent_id: String,
}

impl AgentIdRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self {
            agent_id: Fields(body).required_text("agent_id")?,
        })
    }
}

#[derive(Debug)]
pub struct CreateProjectRequest {
    pub name: String,
    pub folder: String,
}

impl CreateProjectRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        Ok(Self {
            name: fields.required_text("name")?,
            folder: fields.required_text("folder")?,
        })
    }
}

#[derive(Debug)]
pub struct ProjectIdRequest {
    pub project_id: String,
}

impl ProjectIdRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self {
            project_id: Fields(body).required_text("project_id")?,
        })
    }
}

/// `settings.get`.
#[derive(Debug)]
pub struct SettingKeyRequest {
    pub key: String,
}

impl SettingKeyRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self {
            key: Fields(body).required_text("key")?,
        })
    }
}

/// `settings.set`.
#[derive(Debug)]
pub struct SettingRequest {
    pub key: String,
    pub value: String,
}

impl SettingRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        Ok(Self {
            key: fields.required_text("key")?,
            value: fields.required_text("value")?,
        })
    }
}
