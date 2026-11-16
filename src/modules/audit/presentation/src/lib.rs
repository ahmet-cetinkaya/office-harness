// SPDX-License-Identifier: AGPL-3.0-or-later

//! Audit module protocol surface: the `agent.actions` request and the
//! `action.recorded` event.

mod handlers;
mod mappings;

pub use handlers::{ACTIONS_REQUEST, AuditHandlers};
pub use mappings::{action_entry_json, action_recorded_event};
