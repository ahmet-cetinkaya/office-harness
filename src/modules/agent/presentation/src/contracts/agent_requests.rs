// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::additions_input::AdditionsInput;
use serde_json::{Map, Value};

use super::{Fields, read_additions};

/// The `role` object of `agent.create` (contract 0002, FR-003). Absent means
/// the no-role behavior of 0001 (PB-1).
#[derive(Debug)]
pub struct RolePlacement {
    pub kind: String,
    pub parent_agent_id: Option<String>,
    pub team_id: Option<String>,
}

/// `agent.create`.
#[derive(Debug)]
pub struct CreateAgentRequest {
    pub name: String,
    pub command: String,
    pub working_folder: Option<String>,
    pub project_id: Option<String>,
    pub additions: Option<AdditionsInput>,
    pub role: Option<RolePlacement>,
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
            additions: match body.get("additions") {
                None | Some(Value::Null) => None,
                Some(value) => Some(read_additions(value)?),
            },
            role: match body.get("role") {
                None | Some(Value::Null) => None,
                Some(value) => Some(read_role(value)?),
            },
        })
    }
}

/// Reads the `role` object: `kind` names the role, the other two fields are
/// optional ids. Every failure names the field at fault.
///
/// # Errors
/// The text names the field at fault.
fn read_role(value: &Value) -> Result<RolePlacement, String> {
    let Value::Object(role) = value else {
        return Err("role must be an object".to_owned());
    };
    let fields = Fields(role);
    Ok(RolePlacement {
        kind: fields
            .required_text("kind")
            .map_err(|error| format!("role.{error}"))?,
        parent_agent_id: fields
            .optional_text("parent_agent_id")
            .map_err(|error| format!("role.{error}"))?,
        team_id: fields
            .optional_text("team_id")
            .map_err(|error| format!("role.{error}"))?,
    })
}

/// `agent.delete`, `agent.start`, `agent.stop`, `agent.new_task`,
/// `agent.additions.get`, and `session.list`.
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

/// `agent.additions.set`.
#[derive(Debug)]
pub struct AdditionsRequest {
    pub agent_id: String,
    pub additions: AdditionsInput,
}

impl AdditionsRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let additions = body
            .get("additions")
            .ok_or_else(|| "additions is required".to_owned())?;
        Ok(Self {
            agent_id: Fields(body).required_text("agent_id")?,
            additions: read_additions(additions)?,
        })
    }
}

/// `session.continue`.
#[derive(Debug)]
pub struct SessionIdRequest {
    pub session_id: String,
}

impl SessionIdRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self {
            session_id: Fields(body).required_text("session_id")?,
        })
    }
}
