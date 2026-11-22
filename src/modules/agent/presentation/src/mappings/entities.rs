// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::commands::delete_agent::WorktreeOutcome;
use agent_application::common::interfaces::WorktreeInfo;
use agent_application::common::models::{AdditionsView, SessionView};
use agent_application::queries::worktrees::WorktreeStatusView;
use agent_domain::entities::{Agent, Project};
use agent_domain::value_objects::{AgentWorktree, OfficeAdditions, TaskText};
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
        "worktree": agent.worktree.as_ref().map_or(Value::Null, worktree_json),
    })
}

/// `{repository, base, branch, folder}` of an own worktree (0010 FR-003).
#[must_use]
pub fn worktree_json(worktree: &AgentWorktree) -> Value {
    json!({
        "repository": worktree.repository.to_string_lossy(),
        "base": worktree.base,
        "branch": worktree.branch,
        "folder": worktree.folder.to_string_lossy(),
    })
}

/// The `agent.worktree_info` answer (0010 D4).
#[must_use]
pub fn worktree_info_json(info: Option<&WorktreeInfo>) -> Value {
    info.map_or_else(
        || json!({"in_repository": false}),
        |info| {
            json!({
                "in_repository": true,
                "base": info.base,
                "base_is_branch": info.base_is_branch,
                "branches": info.branches,
                "branch_name": info.branch_name,
                "subfolder": info.subfolder,
            })
        },
    )
}

/// One `agent.worktree_status` entry (0010 D6).
#[must_use]
pub fn worktree_status_json(view: &WorktreeStatusView) -> Value {
    json!({
        "agent_id": view.agent.id.to_string(),
        "name": view.agent.name.as_str(),
        "folder": view.worktree.folder.to_string_lossy(),
        "branch": view.worktree.branch,
        "base": view.worktree.base,
        "folder_exists": view.status.folder_exists,
        "reachable": view.status.reachable,
        "changed_files": view.status.changed_files,
        "status_hash": view.status.status_hash,
        "own_commits": view.status.own_commits,
        "base_exists": view.status.base_exists,
    })
}

/// The `agent.delete` answer: what happened to the worktrees (0010 FR-007).
#[must_use]
pub fn worktree_outcome_json(outcome: &WorktreeOutcome) -> Value {
    json!({
        "deleted_branches": outcome.deleted_branches,
        "kept_branches": outcome.kept_branches,
        "kept_folders": outcome.kept_folders,
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
