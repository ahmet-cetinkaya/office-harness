// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::errors::NotificationError;

/// The event that a notification tells the user about (D4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationEvent {
    Exited,
    Failed,
    WaitingForInput,
    Bell,
    AutoHold,
    /// A resume that failed twice put the agent on hold (0013).
    ResumeHold,
}

impl NotificationEvent {
    pub const ALL: [Self; 6] = [
        Self::Exited,
        Self::Failed,
        Self::WaitingForInput,
        Self::Bell,
        Self::AutoHold,
        Self::ResumeHold,
    ];

    /// The stored and wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exited => "exited",
            Self::Failed => "failed",
            Self::WaitingForInput => "waiting_for_input",
            Self::Bell => "bell",
            Self::AutoHold => "auto_hold",
            Self::ResumeHold => "resume_hold",
        }
    }
}

impl fmt::Display for NotificationEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for NotificationEvent {
    type Err = NotificationError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|event| event.as_str() == text)
            .ok_or_else(|| NotificationError(format!(
                "notification event is not one of exited, failed, waiting_for_input, bell, auto_hold, resume_hold: {text}"
            )))
    }
}
