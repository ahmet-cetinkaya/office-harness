// SPDX-License-Identifier: AGPL-3.0-or-later

use schedule_domain::{FieldError, OffReason};
use thiserror::Error;

/// Why a switch on is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SwitchRefusal {
    #[error("its once time has passed; edit the schedule to name a future time")]
    OnceTimePassed,
    #[error(
        "the office switched it off because its target was deleted; edit it to name another target"
    )]
    TargetDeleted,
    #[error("the office switched it off because its time was missed; edit it to name a new time")]
    TimeMissed,
}

impl From<OffReason> for SwitchRefusal {
    fn from(reason: OffReason) -> Self {
        match reason {
            OffReason::TargetDeleted => Self::TargetDeleted,
            OffReason::TimeMissed => Self::TimeMissed,
        }
    }
}

/// What a schedule use case can fail with, mapped to the wire error codes.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ScheduleApplicationError {
    /// Each failing field and its rule; nothing was kept.
    #[error("{} field(s) break a rule", .0.len())]
    Invalid(Vec<FieldError>),
    #[error("the office already has {cap} schedules, the schedule cap")]
    CapReached { cap: u32 },
    #[error("a switch on is refused: {0}")]
    SwitchRefused(SwitchRefusal),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    RunSettled(String),
    #[error("schedule name \"{name}\" is already used")]
    NameTaken { name: String },
    #[error("storage failure: {message}")]
    Storage {
        message: String,
        /// True when the disk, not the request, is at fault.
        is_disk_failure: bool,
    },
}

impl ScheduleApplicationError {
    #[must_use]
    pub const fn is_disk_failure(&self) -> bool {
        matches!(
            self,
            Self::Storage {
                is_disk_failure: true,
                ..
            }
        )
    }
}
