// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

use crate::errors::AgentError;

const DEFAULT_QUIET_PERIOD_MS: u64 = 5_000;
const DEFAULT_OUTPUT_LINE_LIMIT: u64 = 10_000;
const DEFAULT_STOP_GRACE_MS: u64 = 10_000;
const DEFAULT_FOCUS_SHORTCUT: &str = "Ctrl+Alt+Escape";

/// The office settings, stored as key and value rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub quiet_period_ms: u64,
    pub output_line_limit: u64,
    pub focus_shortcut: String,
    pub stop_grace_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingKey {
    QuietPeriodMs,
    OutputLineLimit,
    FocusShortcut,
    StopGraceMs,
}

impl SettingKey {
    pub const ALL: [Self; 4] = [
        Self::QuietPeriodMs,
        Self::OutputLineLimit,
        Self::FocusShortcut,
        Self::StopGraceMs,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::QuietPeriodMs => "quiet_period_ms",
            Self::OutputLineLimit => "output_line_limit",
            Self::FocusShortcut => "focus_shortcut",
            Self::StopGraceMs => "stop_grace_ms",
        }
    }

    /// # Errors
    /// `UnknownSetting` when no setting has this name.
    pub fn parse(name: &str) -> Result<Self, AgentError> {
        Self::ALL
            .into_iter()
            .find(|key| key.as_str() == name)
            .ok_or_else(|| AgentError::UnknownSetting(name.to_owned()))
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            quiet_period_ms: DEFAULT_QUIET_PERIOD_MS,
            output_line_limit: DEFAULT_OUTPUT_LINE_LIMIT,
            focus_shortcut: DEFAULT_FOCUS_SHORTCUT.to_owned(),
            stop_grace_ms: DEFAULT_STOP_GRACE_MS,
        }
    }
}

impl Settings {
    /// The stored text of one setting.
    #[must_use]
    pub fn value_of(&self, key: SettingKey) -> String {
        match key {
            SettingKey::QuietPeriodMs => self.quiet_period_ms.to_string(),
            SettingKey::OutputLineLimit => self.output_line_limit.to_string(),
            SettingKey::FocusShortcut => self.focus_shortcut.clone(),
            SettingKey::StopGraceMs => self.stop_grace_ms.to_string(),
        }
    }

    /// Sets one setting from its stored text.
    ///
    /// # Errors
    /// `InvalidNumber`, `NotPositive`, or `Empty`, naming the setting.
    pub fn set_from_text(&mut self, key: SettingKey, text: &str) -> Result<(), AgentError> {
        match key {
            SettingKey::QuietPeriodMs => self.quiet_period_ms = parse_positive(key, text)?,
            SettingKey::OutputLineLimit => self.output_line_limit = parse_positive(key, text)?,
            SettingKey::StopGraceMs => self.stop_grace_ms = parse_positive(key, text)?,
            SettingKey::FocusShortcut => {
                if text.trim().is_empty() {
                    return Err(AgentError::Empty {
                        field: key.as_str(),
                    });
                }
                text.clone_into(&mut self.focus_shortcut);
            }
        }
        Ok(())
    }
}

fn parse_positive(key: SettingKey, text: &str) -> Result<u64, AgentError> {
    let field = key.as_str();
    let number: u64 = text.parse().map_err(|_| AgentError::InvalidNumber {
        field,
        text: text.to_owned(),
    })?;
    if number == 0 {
        return Err(AgentError::NotPositive { field });
    }
    Ok(number)
}

#[cfg(test)]
#[path = "settings.tests.rs"]
mod tests;
