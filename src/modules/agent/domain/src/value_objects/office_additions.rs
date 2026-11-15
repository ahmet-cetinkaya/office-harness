// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::errors::AgentError;
use crate::value_objects::EnvEntry;

/// What office-harness adds to one agent's CLI start: extra arguments,
/// environment variables, and a startup prompt (FR-020, FR-022).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OfficeAdditions {
    flags: Vec<String>,
    env: Vec<EnvEntry>,
    startup_prompt: Option<String>,
}

impl OfficeAdditions {
    /// A blank prompt counts as none.
    ///
    /// # Errors
    /// `Empty` naming the flag that is empty, and `ContainsNul` for a flag
    /// or a prompt with a NUL character.
    pub fn new(
        flags: Vec<String>,
        env: Vec<EnvEntry>,
        startup_prompt: Option<String>,
    ) -> Result<Self, AgentError> {
        for (index, flag) in flags.iter().enumerate() {
            if flag.is_empty() {
                return Err(AgentError::EmptyFlag { index });
            }
            reject_nul(flag, || format!("flag {}", index + 1))?;
        }
        let startup_prompt = startup_prompt.filter(|prompt| !prompt.trim().is_empty());
        if let Some(prompt) = &startup_prompt {
            reject_nul(prompt, || "startup prompt".to_owned())?;
        }
        Ok(Self {
            flags,
            env,
            startup_prompt,
        })
    }

    #[must_use]
    pub fn flags(&self) -> &[String] {
        &self.flags
    }

    #[must_use]
    pub fn env(&self) -> &[EnvEntry] {
        &self.env
    }

    #[must_use]
    pub fn startup_prompt(&self) -> Option<&str> {
        self.startup_prompt.as_deref()
    }
}

fn reject_nul(text: &str, field: impl FnOnce() -> String) -> Result<(), AgentError> {
    if text.contains('\0') {
        return Err(AgentError::ContainsNul { field: field() });
    }
    Ok(())
}

#[cfg(test)]
#[path = "office_additions.tests.rs"]
mod tests;
