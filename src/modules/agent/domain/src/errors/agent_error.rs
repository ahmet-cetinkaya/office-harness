// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

use crate::enums::{AgentStatus, StatusEvent};

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
    #[error("unknown agent status: {0}")]
    UnknownStatus(String),
    #[error("unknown setting: {0}")]
    UnknownSetting(String),
    #[error("an agent that is {from} cannot take the event {event}")]
    InvalidTransition {
        from: AgentStatus,
        event: StatusEvent,
    },
}
