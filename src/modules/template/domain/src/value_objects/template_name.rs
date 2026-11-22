// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use crate::errors::TemplateError;

const MAX_NAME_CHARS: usize = 100;

/// A template name: trimmed, not blank, no control character. Two names are
/// the same template name when their lower-cased forms (the key) are equal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateName(String);

impl TemplateName {
    /// # Errors
    /// `NameEmpty`, `NameTooLong`, or `NameHasControlCharacter`.
    pub fn parse(text: &str) -> Result<Self, TemplateError> {
        let name = text.trim();
        if name.is_empty() {
            return Err(TemplateError::NameEmpty);
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(TemplateError::NameTooLong {
                max: MAX_NAME_CHARS,
            });
        }
        if name.chars().any(char::is_control) {
            return Err(TemplateError::NameHasControlCharacter);
        }
        Ok(Self(name.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The text the unique index compares.
    #[must_use]
    pub fn key(&self) -> String {
        self.0.to_lowercase()
    }

    /// The name with ` (<number>)` added (A3). The base is cut so that the
    /// result still fits the length rule.
    #[must_use]
    pub fn numbered(&self, number: u32) -> Self {
        let suffix = format!(" ({number})");
        let room = MAX_NAME_CHARS.saturating_sub(suffix.chars().count());
        let base: String = self.0.chars().take(room).collect();
        Self(format!("{base}{suffix}"))
    }
}

impl fmt::Display for TemplateName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
#[path = "template_name.tests.rs"]
mod tests;
