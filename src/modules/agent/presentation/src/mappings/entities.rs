// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::models::{AdditionsView, SessionView};
use agent_domain::entities::{Agent, Project};
use agent_domain::value_objects::{OfficeAdditions, TaskText};
use serde_json::{Value, json};

#[must_use]
pub fn agent_json(agent: &Agent) -> Value {
    // `last_line` stays null here because the entity holds no terminal line;
    // the key is present so the client reads one shape (panel-protocol.md),
    // and `agent.list` answers overwrite it with the terminal's real line.
    json!({
        "id": agent.id.to_string(),
        "name": agent.name.as_str(),
        "command": agent.command.as_str(),
        "working_folder": agent.working_folder.as_ref().map(|folder| folder.to_string_lossy()),
        "project_id": agent.project_id.map(|id| id.to_string()),
        "machine_id": agent.machine_id.to_string(),
        "status": agent.status.as_str(),
        "created_at": agent.created_at.as_unix_millis(),
        "last_line": Value::Null,
        "task": task_json(agent.task.as_ref()),
    })
}

/// `{text, set_at, source}`, or `null` when the agent has no task text.
#[must_use]
pub fn task_json(task: Option<&TaskText>) -> Value {
    task.map_or(Value::Null, |task| {
        json!({
            "text": task.text(),
            "set_at": task.set_at().as_unix_millis(),
            "source": task.source().as_str(),
        })
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

fn additions_body(additions: &OfficeAdditions) -> Value {
    json!({
        "flags": additions.flags(),
        "env": additions
            .env()
            .iter()
            .map(|entry| json!({"name": entry.name(), "value": entry.value()}))
            .collect::<Vec<_>>(),
        "startup_prompt": additions.startup_prompt(),
    })
}

/// The additions, and whether an edit waits for the next session.
#[must_use]
pub fn additions_json(view: &AdditionsView) -> Value {
    let mut body = additions_body(&view.record.additions);
    body["updated_at"] = json!(view.record.updated_at.as_unix_millis());
    body["applies_next_session"] = json!(view.applies_next_session);
    body
}

/// A session with `can_continue` and, when it is false, `reason`.
#[must_use]
pub fn session_json(view: &SessionView) -> Value {
    let session = &view.session;
    let mut body = json!({
        "id": session.id.to_string(),
        "agent_id": session.agent_id.to_string(),
        "cli_session_id": session.cli_session_id,
        "continued_from": session.continued_from.map(|id| id.to_string()),
        "additions_snapshot": additions_body(&session.additions_snapshot),
        "started_at": session.started_at.as_unix_millis(),
        "ended_at": session.ended_at.map(kernel::Timestamp::as_unix_millis),
        "exit_result": session.exit_result.as_ref().map(ToString::to_string),
        "can_continue": view.can_continue(),
    });
    if let Some(reason) = &view.cannot_continue_reason {
        body["reason"] = json!(reason);
    }
    body
}
