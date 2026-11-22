// SPDX-License-Identifier: AGPL-3.0-or-later

mod agent_command;
mod agent_worktree;
mod display_name;
mod entity_ids;
mod env_entry;
mod exit_result;
mod office_additions;
mod settings;
mod task_text;

pub use agent_command::AgentCommand;
pub use agent_worktree::{
    AgentWorktree, BRANCH_PREFIX, base_ref, branch_slug, free_branch_name, is_commit_id,
    is_office_branch_name,
};
pub use display_name::DisplayName;
pub use entity_ids::{MachineId, SessionId};
pub use env_entry::{EnvEntry, RESERVED_ENV_PREFIX};
pub use exit_result::ExitResult;
// The project id belongs to the shared kernel; the agent module re-exports it.
pub use kernel::ProjectId;
pub use office_additions::OfficeAdditions;
pub use settings::{SettingKey, Settings};
pub use task_text::{TASK_TEXT_MAX_CHARS, TaskSource, TaskText};
