// SPDX-License-Identifier: AGPL-3.0-or-later

//! Agent module protocol surface: the `agent.*` (except `agent.actions`, which
//! the audit module answers), `project.*`, and `settings.*` requests, and
//! the `agent.status`, `agent.changed`, and `agent.deleted` events.

pub mod contracts;
pub mod handlers;
pub mod mappings;

pub use handlers::{
    AgentHandlers, AgentUseCases, Handled, REQUEST_PREFIXES, SubtreeRequests, SubtreeResult,
};
pub use mappings::{agent_line_event, agent_status_event};
