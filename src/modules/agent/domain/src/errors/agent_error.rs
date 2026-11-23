// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

use crate::enums::{AgentStatus, StatusEvent};
use crate::value_objects::RESERVED_ENV_PREFIX;

/// A rule of the agent module that an input or a transition broke. Messages
/// name the field at fault.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AgentError {
    #[error("{field} must not be empty")]
    Empty { field: &'static str },
    #[error("{field} must be at most {max} characters")]
    TooLong { field: &'static str, max: usize },
    #[error("{field} must be an absolute path")]
    RelativePath { field: &'static str },
    #[error("an agent needs a working folder or a project")]
    NoWorkingFolderOrProject,
    #[error("{field} is not a valid identifier: {text}")]
    InvalidId { field: &'static str, text: String },
    #[error("{field} is not a valid number: {text}")]
    InvalidNumber { field: &'static str, text: String },
    #[error("{field} must be greater than zero")]
    NotPositive { field: &'static str },
    #[error("{field}: a limit is a whole number of at least 1 and at most 1000")]
    InvalidLimit { field: &'static str },
    #[error("{field} is 0 (off) or a whole number of minutes from 5 to 1440")]
    InvalidIdleSleep { field: &'static str },
    #[error(
        "environment entry {name:?} is malformed: a name starts with a letter or an underscore \
         and holds only letters, digits, and underscores"
    )]
    InvalidEnvName { name: String },
    #[error(
        "environment entry {name:?} is reserved for office-harness: names that start with {RESERVED_ENV_PREFIX} are set by the server"
    )]
    ReservedEnvName { name: String },
    #[error("flag {} must not be empty", .index + 1)]
    EmptyFlag { index: usize },
    #[error("{field} must not contain a NUL character")]
    ContainsNul { field: String },
    #[error("{field} must not contain control characters other than tab and line feed")]
    ContainsControlCharacter { field: String },
    #[error("unknown exit result: {0}")]
    UnknownExitResult(String),
    #[error("a task text has at most 8,192 characters")]
    TaskTextTooLong,
    #[error("unknown task source: {0}")]
    UnknownTaskSource(String),
    #[error("unknown agent status: {0}")]
    UnknownStatus(String),
    #[error("unknown setting: {0}")]
    UnknownSetting(String),
    #[error(
        "{field} must be JSON like {{\"project\": string or null, \"agent\": string or null, \
         \"terminal_open\": boolean}}: {problem}"
    )]
    InvalidLastView {
        field: &'static str,
        problem: String,
    },
    #[error("{field} must be at most {max} bytes")]
    TooLarge { field: &'static str, max: usize },
    #[error("{field} {shortcut} is reserved for {reserved_for}")]
    ReservedShortcut {
        field: &'static str,
        shortcut: &'static str,
        reserved_for: &'static str,
    },
    #[error("{field} {reason}")]
    InvalidFilePath {
        field: &'static str,
        reason: &'static str,
    },
    #[error("an agent that is {from} cannot take the event {event}")]
    InvalidTransition {
        from: AgentStatus,
        event: StatusEvent,
    },
}
