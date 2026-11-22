// SPDX-License-Identifier: AGPL-3.0-or-later

mod fields;
mod template_requests;

pub use fields::Fields;
pub use template_requests::{
    CREATE_AGENT, CREATE_TEAM, CreateAgentFields, CreateTeamFields, DELETE, EXPORT, ExportFields,
    FROM_AGENT, FromAgentFields, GET, IMPORT_CONFIRM, IMPORT_PREVIEW, ImportFields, LIST,
    PlaceFields, SAVE, SaveFields, TemplateIdFields, refuse_unknown_fields,
};
