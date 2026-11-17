// SPDX-License-Identifier: AGPL-3.0-or-later

mod agent_command;
mod display_name;
mod entity_ids;
mod env_entry;
mod exit_result;
mod office_additions;
mod settings;

pub use agent_command::AgentCommand;
pub use display_name::DisplayName;
pub use entity_ids::{MachineId, SessionId};
pub use env_entry::{EnvEntry, RESERVED_ENV_PREFIX};
pub use exit_result::ExitResult;
// The project id belongs to the shared kernel; the agent module re-exports it.
pub use kernel::ProjectId;
pub use office_additions::OfficeAdditions;
pub use settings::{SettingKey, Settings};
