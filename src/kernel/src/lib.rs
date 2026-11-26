// SPDX-License-Identifier: AGPL-3.0-or-later

//! Shared kernel: the few domain types that several modules agree on.

mod actor;
mod agent_id;
mod project_id;
mod schedule_id;
mod text_redactor;
mod timestamp;
mod visible_text;

pub use actor::Actor;
pub use agent_id::{AgentId, AgentIdError};
pub use project_id::{ProjectId, ProjectIdError};
pub use schedule_id::{ScheduleId, ScheduleIdError};
pub use text_redactor::{NoRedaction, StreamRedactor, TextRedactor};
pub use timestamp::Timestamp;
pub use visible_text::visible_text;
