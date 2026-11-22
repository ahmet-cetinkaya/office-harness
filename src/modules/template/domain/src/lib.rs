// SPDX-License-Identifier: AGPL-3.0-or-later

//! Template module domain: saved agent and team definitions, the name
//! pattern rules, and the template file format. No I/O, and no knowledge of
//! the agent or hierarchy modules: a template holds plain text and numbers.

mod entities;
mod enums;
mod errors;
mod services;
mod value_objects;

pub use entities::{
    AdditionsBody, AgentBody, EnvVariable, MAX_TEAM_AGENTS, MemberEntry, TeamBody, Template,
    TemplateBody,
};
pub use enums::{TemplateKind, TemplateRole};
pub use errors::TemplateError;
pub use services::{
    FileTemplate, MAX_FILE_BYTES, body_to_json, parse_body, parse_file, render_file, save_name,
};
pub use value_objects::{
    LEAD_PATTERN, MEMBER_PATTERN, NamePattern, TemplateId, TemplateName, default_agent_pattern,
};
