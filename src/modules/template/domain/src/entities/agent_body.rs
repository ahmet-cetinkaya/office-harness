// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::enums::TemplateRole;
use crate::errors::TemplateError;
use crate::value_objects::NamePattern;

/// The longest agent name the office accepts (0001).
const MAX_AGENT_NAME_CHARS: usize = 64;

/// One environment variable of a template. `None` is "no value": the CLI does
/// not get the variable (D3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvVariable {
    pub name: String,
    pub value: Option<String>,
}

/// The office additions of a template, as text. The agent module's rules
/// check them when an agent is created from the template.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdditionsBody {
    pub flags: Vec<String>,
    pub env: Vec<EnvVariable>,
    pub startup_prompt: Option<String>,
}

impl AdditionsBody {
    /// The same additions with every environment value dropped (D3, A6).
    #[must_use]
    pub fn without_env_values(mut self) -> Self {
        for variable in &mut self.env {
            variable.value = None;
        }
        self
    }
}

/// A saved agent definition (D2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentBody {
    pub name_pattern: NamePattern,
    pub command: String,
    pub working_folder: Option<String>,
    pub additions: AdditionsBody,
    pub role: Option<TemplateRole>,
}

impl AgentBody {
    /// The FR-001 save rules. `place` names the entry in the refusal.
    ///
    /// # Errors
    /// `CommandEmpty`, `PatternEmpty`, or `PatternTooLong`.
    pub fn validate(&self, place: &str) -> Result<(), TemplateError> {
        if self.command.trim().is_empty() {
            return Err(TemplateError::CommandEmpty {
                place: place.to_owned(),
            });
        }
        if self.name_pattern.is_empty() {
            return Err(TemplateError::PatternEmpty {
                place: place.to_owned(),
            });
        }
        if self.name_pattern.least_expanded_chars() > MAX_AGENT_NAME_CHARS {
            return Err(TemplateError::PatternTooLong {
                place: place.to_owned(),
                pattern: self.name_pattern.as_str().to_owned(),
                max: MAX_AGENT_NAME_CHARS,
            });
        }
        Ok(())
    }

    #[must_use]
    pub fn without_env_values(mut self) -> Self {
        self.additions = self.additions.without_env_values();
        self
    }
}
