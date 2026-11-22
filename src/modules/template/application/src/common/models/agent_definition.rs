// SPDX-License-Identifier: AGPL-3.0-or-later

use template_domain::{AdditionsBody, TemplateRole};

/// What an existing agent holds that a template copies (FR-001).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDefinition {
    pub name: String,
    pub command: String,
    pub working_folder: Option<String>,
    pub additions: AdditionsBody,
    pub role: Option<TemplateRole>,
}
