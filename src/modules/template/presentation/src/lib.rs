// SPDX-License-Identifier: AGPL-3.0-or-later

//! Template module protocol surface: the `template.*` requests the module
//! answers itself, the readers of the two create requests that the host
//! answers (they cross modules), and the `template.changed` event (0009 D7).

pub mod contracts;
pub mod handlers;
pub mod mappings;

pub use contracts::{
    CREATE_AGENT, CREATE_TEAM, CreateAgentFields, CreateTeamFields, PlaceFields,
    refuse_unknown_fields,
};
pub use handlers::{Handled, HandlerError, REQUEST_PREFIX, TemplateHandlers};
pub use mappings::{template_changed_event, template_json};
