// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::errors::AgentError;
use crate::value_objects::EnvEntry;

/// The control characters a pasted prompt may keep: a tab and a line break.
const ALLOWED_PROMPT_CONTROLS: [char; 2] = ['\t', '\n'];

/// What office-harness adds to one agent's CLI start: extra arguments,
/// environment variables, and a startup prompt (FR-020, FR-022).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OfficeAdditions {
    flags: Vec<String>,
    env: Vec<EnvEntry>,
    startup_prompt: Option<String>,
}

impl OfficeAdditions {
    /// A blank prompt counts as none, and a prompt loses its trailing line
    /// breaks: it is pasted and never submitted, so a final break would only
    /// leave an empty line.
    ///
    /// # Errors
    /// `EmptyFlag` naming the flag that is empty, `ContainsNul` for a flag
    /// with a NUL character, and `ContainsControlCharacter` for a prompt
    /// with a control character other than tab and line feed (an ESC or a
    /// carriage return would act as a key press in the terminal).
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
        let startup_prompt = startup_prompt
            .map(|prompt| prompt.trim_end_matches(['\r', '\n']).to_owned())
            .filter(|prompt| !prompt.trim().is_empty());
        if let Some(prompt) = &startup_prompt {
            reject_control_characters(prompt, || "startup prompt".to_owned())?;
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

fn reject_control_characters(text: &str, field: impl FnOnce() -> String) -> Result<(), AgentError> {
    if text
        .chars()
        .any(|character| character.is_control() && !ALLOWED_PROMPT_CONTROLS.contains(&character))
    {
        return Err(AgentError::ContainsControlCharacter { field: field() });
    }
    Ok(())
}

#[cfg(test)]
#[path = "office_additions.tests.rs"]
mod tests;
