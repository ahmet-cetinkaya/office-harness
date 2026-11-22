// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;
use std::str::FromStr;

use kernel::Timestamp;

use crate::errors::AgentError;

/// The most characters a task text holds, counted as Unicode scalar values
/// like the directive bound it matches.
pub const TASK_TEXT_MAX_CHARS: usize = 8192;

const DIRECTIVE: &str = "directive";
const USER: &str = "user";

/// Who gave the agent its task text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSource {
    /// A directive that the office entered into the agent's terminal.
    Directive,
    /// The user, in the agent's details.
    User,
}

impl TaskSource {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Directive => DIRECTIVE,
            Self::User => USER,
        }
    }
}

impl fmt::Display for TaskSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for TaskSource {
    type Err = AgentError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            DIRECTIVE => Ok(Self::Directive),
            USER => Ok(Self::User),
            other => Err(AgentError::UnknownTaskSource(other.to_owned())),
        }
    }
}

/// The last task the office gave an agent: the text, when it was set, and by
/// whom. The text is stored as entered, control characters included; showing
/// it safely is the client's rule (FR-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskText {
    text: String,
    set_at: Timestamp,
    source: TaskSource,
}

impl TaskText {
    /// A task text, or `None` when the text is empty or only whitespace, which
    /// clears the task (spec edge case). The blank check runs before the
    /// length check, so a long run of spaces clears instead of being refused.
    ///
    /// # Errors
    /// `TaskTextTooLong` past `TASK_TEXT_MAX_CHARS` characters.
    pub fn from_input(
        text: &str,
        set_at: Timestamp,
        source: TaskSource,
    ) -> Result<Option<Self>, AgentError> {
        if text.trim().is_empty() {
            return Ok(None);
        }
        if text.chars().count() > TASK_TEXT_MAX_CHARS {
            return Err(AgentError::TaskTextTooLong);
        }
        Ok(Some(Self {
            text: text.to_owned(),
            set_at,
            source,
        }))
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub const fn set_at(&self) -> Timestamp {
        self.set_at
    }

    #[must_use]
    pub const fn source(&self) -> TaskSource {
        self.source
    }
}

#[cfg(test)]
#[path = "task_text.tests.rs"]
mod tests;
