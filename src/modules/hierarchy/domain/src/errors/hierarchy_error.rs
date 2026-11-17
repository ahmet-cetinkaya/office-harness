// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

use crate::value_objects::Role;

/// A rule of the hierarchy that an input or a tree change broke. Messages
/// name the field or the agent at fault and stay free of paths.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum HierarchyError {
    #[error("{field} must not be empty")]
    Empty { field: &'static str },
    #[error("{field} must be at most {max} characters")]
    TooLong { field: &'static str, max: usize },
    #[error("{field} must not contain control characters")]
    ControlCharacter { field: &'static str },
    #[error("{field} is not a valid identifier: {text}")]
    InvalidId { field: &'static str, text: String },
    #[error("{field} is not a role of the company: {text}")]
    UnknownRole { field: &'static str, text: String },
    #[error("a team named {name} already exists")]
    TeamNameTaken { name: String },
    #[error("the team does not exist")]
    TeamNotFound,
    #[error("a team needs its lead")]
    LeadMissing,
    #[error("the office already has a CEO: {name}")]
    SecondCeo { name: String },
    #[error("the parent is not an agent of the office")]
    ParentNotFound,
    #[error("a {child} cannot sit under a {parent}; allowed under a {parent}: {allowed}")]
    RoleNotAllowed {
        child: Role,
        parent: Role,
        allowed: String,
    },
    #[error("the parent chain would loop")]
    Cycle,
    #[error("the tree is deeper than three levels")]
    TooDeep,
    #[error("the agent is not part of the hierarchy")]
    NotInHierarchy,
    #[error("the agent is not a descendant of {actor}")]
    NotADescendant { actor: String },
    #[error("the agent is not running: {name}")]
    NotRunning { name: String },
    #[error("the confirmation counted {confirmed} descendants, but there are {actual}")]
    DescendantCountChanged { confirmed: usize, actual: usize },
    #[error("storage failed: {0}")]
    Storage(String),
}
