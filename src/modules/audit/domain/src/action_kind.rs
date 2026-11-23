// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;
use std::str::FromStr;

use crate::ActionError;

/// What a user or an agent did to an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionKind {
    Create,
    Start,
    Stop,
    Delete,
    NewTask,
    Continue,
    TeamCreate,
    TeamDelete,
    Directive,
    Refused,
    Hold,
    Release,
    TaskText,
    HelperLimit,
}

impl ActionKind {
    pub const ALL: [Self; 14] = [
        Self::Create,
        Self::Start,
        Self::Stop,
        Self::Delete,
        Self::NewTask,
        Self::Continue,
        Self::TeamCreate,
        Self::TeamDelete,
        Self::Directive,
        Self::Refused,
        Self::Hold,
        Self::Release,
        Self::TaskText,
        Self::HelperLimit,
    ];

    /// The stored and wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Delete => "delete",
            Self::NewTask => "new_task",
            Self::Continue => "continue",
            Self::TeamCreate => "team_create",
            Self::TeamDelete => "team_delete",
            Self::Directive => "directive",
            Self::Refused => "refused",
            Self::Hold => "hold",
            Self::Release => "release",
            Self::TaskText => "task_text",
            Self::HelperLimit => "helper_limit",
        }
    }
}

impl fmt::Display for ActionKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ActionKind {
    type Err = ActionError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_str() == text)
            .ok_or_else(|| ActionError {
                field: "action",
                text: text.to_owned(),
            })
    }
}
