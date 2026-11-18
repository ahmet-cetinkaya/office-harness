// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::errors::AgentError;

const DEFAULT_QUIET_PERIOD_MS: u64 = 5_000;
const DEFAULT_OUTPUT_LINE_LIMIT: u64 = 10_000;
const DEFAULT_STOP_GRACE_MS: u64 = 10_000;
const DEFAULT_FOCUS_SHORTCUT: &str = "Ctrl+Shift+F12";

/// The longest stored text of one setting, in bytes (0003 data model).
const LAST_VIEW_MAX_BYTES: usize = 1_024;

/// The shortcuts of 0003 A1 that the focus shortcut must not take (D8). The
/// desktop app shows the same table, so the two stay in step.
const RESERVED_SHORTCUTS: [(&str, &str); 10] = [
    ("Ctrl+Shift+F1", "agent 1"),
    ("Ctrl+Shift+F2", "agent 2"),
    ("Ctrl+Shift+F3", "agent 3"),
    ("Ctrl+Shift+F4", "agent 4"),
    ("Ctrl+Shift+F5", "agent 5"),
    ("Ctrl+Shift+F6", "agent 6"),
    ("Ctrl+Shift+F7", "agent 7"),
    ("Ctrl+Shift+F8", "agent 8"),
    ("Ctrl+Shift+F9", "agent 9"),
    ("Ctrl+Shift+F10", "the agent switcher"),
];

/// The reserved table in the form a value is compared in, built once.
static RESERVED_SHORTCUTS_PARSED: LazyLock<Vec<(&'static str, &'static str, String)>> =
    LazyLock::new(|| {
        RESERVED_SHORTCUTS
            .iter()
            .filter_map(|(text, reserved_for)| {
                Some((*text, *reserved_for, normalized_shortcut(text)?))
            })
            .collect()
    });

const KEY_SEPARATOR: char = '+';
const MODIFIER_NAMES: [&str; 5] = ["ctrl", "control", "alt", "shift", "meta"];

/// One key combination in the canonical form `ctrl+shift+f1`, the form the
/// desktop's `parseShortcut` reads: modifiers first, lowercase, trimmed, with
/// `esc` and `space` under their `KeyboardEvent.key` names. Modifier order,
/// spaces around the parts, and case no longer matter. `None` when the text
/// names no key, or only a modifier, which the desktop matches to nothing.
fn normalized_shortcut(text: &str) -> Option<String> {
    let mut parts: Vec<String> = text
        .split(KEY_SEPARATOR)
        .map(|part| part.trim().to_lowercase())
        .collect();
    let key = parts.pop()?;
    if key.is_empty() || MODIFIER_NAMES.contains(&key.as_str()) {
        return None;
    }
    // The desktop reads only these exact tokens as flags; "control" is a
    // modifier name for key rejection, but it does not set the ctrl flag.
    let mut normalized: Vec<String> = ["ctrl", "alt", "shift", "meta"]
        .iter()
        .filter(|name| parts.iter().any(|part| part == **name))
        .map(|name| (*name).to_owned())
        .collect();
    normalized.push(match key.as_str() {
        "esc" => "escape".to_owned(),
        "space" => " ".to_owned(),
        _ => key,
    });
    Some(normalized.join(&KEY_SEPARATOR.to_string()))
}

/// The office settings, stored as key and value rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub quiet_period_ms: u64,
    pub output_line_limit: u64,
    pub focus_shortcut: String,
    pub stop_grace_ms: u64,
    pub last_view: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingKey {
    QuietPeriodMs,
    OutputLineLimit,
    FocusShortcut,
    StopGraceMs,
    LastView,
}

impl SettingKey {
    pub const ALL: [Self; 5] = [
        Self::QuietPeriodMs,
        Self::OutputLineLimit,
        Self::FocusShortcut,
        Self::StopGraceMs,
        Self::LastView,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::QuietPeriodMs => "quiet_period_ms",
            Self::OutputLineLimit => "output_line_limit",
            Self::FocusShortcut => "focus_shortcut",
            Self::StopGraceMs => "stop_grace_ms",
            Self::LastView => "last_view",
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
            last_view: None,
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
            // No stored view is the empty text, which reads back as no view.
            SettingKey::LastView => self.last_view.clone().unwrap_or_default(),
        }
    }

    /// Sets one setting from its stored text.
    ///
    /// # Errors
    /// `InvalidNumber`, `NotPositive`, or `Empty`, naming the setting;
    /// `InvalidLastView`, `TooLarge`, or `ReservedShortcut` for the 0003 keys.
    pub fn set_from_text(&mut self, key: SettingKey, text: &str) -> Result<(), AgentError> {
        match key {
            SettingKey::QuietPeriodMs => self.quiet_period_ms = parse_positive(key, text)?,
            SettingKey::OutputLineLimit => self.output_line_limit = parse_positive(key, text)?,
            SettingKey::StopGraceMs => self.stop_grace_ms = parse_positive(key, text)?,
            SettingKey::FocusShortcut => {
                self.focus_shortcut = parse_shortcut(key, text)?;
            }
            SettingKey::LastView => {
                self.last_view = parse_last_view(key, text)?;
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

fn parse_shortcut(key: SettingKey, text: &str) -> Result<String, AgentError> {
    if text.trim().is_empty() {
        return Err(AgentError::Empty {
            field: key.as_str(),
        });
    }
    let conflict = normalized_shortcut(text).and_then(|combination| {
        RESERVED_SHORTCUTS_PARSED
            .iter()
            .find(|(_, _, normalized)| *normalized == combination)
            .map(|(reserved, reserved_for, _)| (*reserved, *reserved_for))
    });
    if let Some((reserved, reserved_for)) = conflict {
        return Err(AgentError::ReservedShortcut {
            field: key.as_str(),
            shortcut: reserved,
            reserved_for,
        });
    }
    Ok(text.to_owned())
}

fn parse_last_view(key: SettingKey, text: &str) -> Result<Option<String>, AgentError> {
    if text.is_empty() {
        return Ok(None);
    }
    let field = key.as_str();
    if text.len() > LAST_VIEW_MAX_BYTES {
        return Err(AgentError::TooLarge {
            field,
            max: LAST_VIEW_MAX_BYTES,
        });
    }
    let view: serde_json::Value =
        serde_json::from_str(text).map_err(|error| AgentError::InvalidLastView {
            field,
            problem: error.to_string(),
        })?;
    if !holds_the_documented_shape(&view) {
        return Err(AgentError::InvalidLastView {
            field,
            problem: "the value must be an object whose project and agent are text or null and \
                      whose terminal_open is a boolean"
                .to_owned(),
        });
    }
    Ok(Some(text.to_owned()))
}

/// The `last_view` shape of the 0003 data model. Unknown keys are ignored, so
/// a client may store more and a later change may add keys.
fn holds_the_documented_shape(view: &serde_json::Value) -> bool {
    view.as_object().is_some_and(|view| {
        text_or_null(view.get("project"))
            && text_or_null(view.get("agent"))
            && view
                .get("terminal_open")
                .is_some_and(serde_json::Value::is_boolean)
    })
}

fn text_or_null(value: Option<&serde_json::Value>) -> bool {
    value.is_some_and(|value| value.is_null() || value.is_string())
}

#[cfg(test)]
#[path = "settings.tests.rs"]
mod tests;
