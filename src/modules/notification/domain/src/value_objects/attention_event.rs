// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;
use std::str::FromStr;

use kernel::Timestamp;

use crate::errors::NotificationError;

/// What made an agent wait for the user (0008 D1). Same five names as a
/// notification event, kept apart so the queue can change without the
/// notification rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttentionKind {
    Failed,
    WaitingForInput,
    Bell,
    AutoHold,
    Exited,
}

impl AttentionKind {
    pub const ALL: [Self; 5] = [
        Self::Failed,
        Self::WaitingForInput,
        Self::Bell,
        Self::AutoHold,
        Self::Exited,
    ];

    /// The stored and wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Failed => "failed",
            Self::WaitingForInput => "waiting_for_input",
            Self::Bell => "bell",
            Self::AutoHold => "auto_hold",
            Self::Exited => "exited",
        }
    }
}

impl fmt::Display for AttentionKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for AttentionKind {
    type Err = NotificationError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_str() == text)
            .ok_or_else(|| {
                NotificationError(format!(
                    "attention kind is not one of failed, waiting_for_input, bell, auto_hold, exited: {text}"
                ))
            })
    }
}

/// One thing that happened to an agent that the user should look at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttentionEvent {
    pub kind: AttentionKind,
    /// What the event adds to its kind (0008 D3): the quiet period, the exit
    /// code text, or the limit name; `None` for a bell.
    pub detail: Option<String>,
    pub at: Timestamp,
}
