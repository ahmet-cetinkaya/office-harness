// SPDX-License-Identifier: AGPL-3.0-or-later

mod agent_body;
mod team_body;
mod template;
mod template_body;

pub use agent_body::{AdditionsBody, AgentBody, EnvVariable};
pub use team_body::{MAX_TEAM_AGENTS, MemberEntry, TeamBody};
pub use template::Template;
pub use template_body::TemplateBody;
