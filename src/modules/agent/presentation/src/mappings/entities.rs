// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::entities::{Agent, Project};
use serde_json::{Value, json};

#[must_use]
pub fn agent_json(agent: &Agent) -> Value {
    json!({
        "id": agent.id.to_string(),
        "name": agent.name.as_str(),
        "command": agent.command.as_str(),
        "working_folder": agent.working_folder.as_ref().map(|folder| folder.to_string_lossy()),
        "project_id": agent.project_id.map(|id| id.to_string()),
        "machine_id": agent.machine_id.to_string(),
        "status": agent.status.as_str(),
        "created_at": agent.created_at.as_unix_millis(),
    })
}

#[must_use]
pub fn project_json(project: &Project) -> Value {
    json!({
        "id": project.id.to_string(),
        "name": project.name.as_str(),
        "folder": project.folder.to_string_lossy(),
    })
}

#[must_use]
pub fn setting_json(key: &str, value: &str) -> Value {
    json!({"key": key, "value": value})
}
