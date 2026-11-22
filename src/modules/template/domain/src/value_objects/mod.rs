// SPDX-License-Identifier: AGPL-3.0-or-later

mod name_pattern;
mod template_id;
mod template_name;

pub use name_pattern::{LEAD_PATTERN, MEMBER_PATTERN, NamePattern, default_agent_pattern};
pub use template_id::TemplateId;
pub use template_name::TemplateName;
