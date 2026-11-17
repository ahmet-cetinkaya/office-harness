// SPDX-License-Identifier: AGPL-3.0-or-later

//! Shared kernel: the few domain types that several modules agree on.

mod actor;
mod agent_id;
mod project_id;
mod timestamp;

pub use actor::Actor;
pub use agent_id::{AgentId, AgentIdError};
pub use project_id::{ProjectId, ProjectIdError};
pub use timestamp::Timestamp;
