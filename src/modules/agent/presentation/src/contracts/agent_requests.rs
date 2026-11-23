// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use agent_application::commands::create_agent::OwnWorktree;
use agent_application::commands::delete_agent::DeleteWorktrees;
use agent_application::common::additions_input::AdditionsInput;
use kernel::AgentId;
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
    /// The own worktree choice (0010); `None` without `own_worktree: true`.
    pub worktree: Option<OwnWorktree>,
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
            worktree: read_own_worktree(&fields)?,
        })
    }
}

/// `own_worktree`, `base`, and `branch_name` (0010 D5). The two names are
/// taken only with the choice on, so a stray one never acts on its own.
fn read_own_worktree(fields: &Fields<'_>) -> Result<Option<OwnWorktree>, String> {
    let base = fields.optional_text("base")?;
    let branch_name = fields.optional_text("branch_name")?;
    if fields.optional_bool("own_worktree")? == Some(true) {
        return Ok(Some(OwnWorktree { base, branch_name }));
    }
    match (&base, &branch_name) {
        (None, None) => Ok(None),
        (Some(_), _) => Err("base is only taken with own_worktree".to_owned()),
        (None, Some(_)) => Err("branch_name is only taken with own_worktree".to_owned()),
    }
}

/// `agent.worktree_info` (0010 D4).
#[derive(Debug)]
pub struct WorktreeInfoRequest {
    pub folder: Option<String>,
    pub project_id: Option<String>,
    pub name: String,
}

impl WorktreeInfoRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        Ok(Self {
            folder: fields.optional_text("folder")?,
            project_id: fields.optional_text("project_id")?,
            name: fields.required_text("name")?,
        })
    }
}

/// `agent.worktree_status` (0010 D6).
#[derive(Debug)]
pub struct WorktreeStatusRequest {
    pub agent_ids: Vec<AgentId>,
}

impl WorktreeStatusRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let Some(Value::Array(ids)) = body.get("agent_ids") else {
            return Err("agent_ids must be a list of agent ids".to_owned());
        };
        let agent_ids = ids
            .iter()
            .map(|id| id.as_str().and_then(AgentId::from_uuid_text))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| "agent_ids must be a list of agent ids".to_owned())?;
        Ok(Self { agent_ids })
    }
}

/// `agent.delete`: the agent and the worktree choice (0010 D6).
#[derive(Debug)]
pub struct DeleteAgentRequest {
    pub agent_id: String,
    pub worktrees: DeleteWorktrees,
}

impl DeleteAgentRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        let worktrees = match fields.optional_text("worktrees")?.as_deref() {
            None | Some("keep") => DeleteWorktrees::Keep,
            Some("remove") => DeleteWorktrees::Remove(read_confirmed_status(body)?),
            Some(_) => return Err("worktrees must be keep or remove".to_owned()),
        };
        Ok(Self {
            agent_id: fields.required_text("agent_id")?,
            worktrees,
        })
    }
}

/// `confirmed_status`: the status hash the question showed, per agent id.
fn read_confirmed_status(body: &Map<String, Value>) -> Result<HashMap<AgentId, String>, String> {
    const SHAPE: &str = "confirmed_status must map agent ids to status hashes";
    match body.get("confirmed_status") {
        None | Some(Value::Null) => Ok(HashMap::new()),
        Some(Value::Object(entries)) => entries
            .iter()
            .map(|(id, hash)| {
                let id = AgentId::from_uuid_text(id).ok_or_else(|| SHAPE.to_owned())?;
                let hash = hash.as_str().ok_or_else(|| SHAPE.to_owned())?;
                Ok((id, hash.to_owned()))
            })
            .collect(),
        Some(_) => Err(SHAPE.to_owned()),
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

/// `agent.resume`: the agent, and for the fallback step the session that the
/// CLI refused.
#[derive(Debug)]
pub struct ResumeAgentRequest {
    pub agent_id: String,
    pub after_session: Option<String>,
}

impl ResumeAgentRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        fields.refuse_other_than("agent.resume", &["agent_id", "after_session"])?;
        Ok(Self {
            agent_id: fields.required_text("agent_id")?,
            after_session: fields.optional_text("after_session")?,
        })
    }
}

/// `agent.wake`: the agent to wake, nothing else.
#[derive(Debug)]
pub struct WakeAgentRequest {
    pub agent_id: String,
}

impl WakeAgentRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        fields.refuse_other_than("agent.wake", &["agent_id"])?;
        Ok(Self {
            agent_id: fields.required_text("agent_id")?,
        })
    }
}

/// `agent.sleep`: the agent and the idle time that the record keeps as the
/// reason. Only the office sends it.
#[derive(Debug)]
pub struct SleepAgentRequest {
    pub agent_id: String,
    pub idle_minutes: u32,
}

impl SleepAgentRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        fields.refuse_other_than("agent.sleep", &["agent_id", "idle_minutes"])?;
        Ok(Self {
            agent_id: fields.required_text("agent_id")?,
            idle_minutes: fields
                .optional_u32("idle_minutes")?
                .ok_or_else(|| "idle_minutes is required".to_owned())?,
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

/// `agent.task.set`. An empty `text` is valid: it clears the task.
#[derive(Debug)]
pub struct TaskTextRequest {
    pub agent_id: String,
    pub text: String,
}

impl TaskTextRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        Ok(Self {
            agent_id: fields.required_text("agent_id")?,
            text: fields.required_text("text")?,
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
