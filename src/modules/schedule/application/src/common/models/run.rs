// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::{ScheduleId, Timestamp};

/// How many runs the history of one schedule keeps.
pub const HISTORY_LIMIT: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RunId(pub i64);

/// What a run did. Every result other than `Submitted` and `Waiting` has a
/// reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunResult {
    Submitted,
    Waiting,
    Unsubmitted,
    Withdrawn,
    Refused,
    Skipped,
    Failed,
    Missed,
}

impl RunResult {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Submitted => "submitted",
            Self::Waiting => "waiting",
            Self::Unsubmitted => "unsubmitted",
            Self::Withdrawn => "withdrawn",
            Self::Refused => "refused",
            Self::Skipped => "skipped",
            Self::Failed => "failed",
            Self::Missed => "missed",
        }
    }

    /// Whether a run that is open (`None`) or has the result `current` can
    /// take this result: an open run takes any result; a run that waits
    /// takes `submitted`, `unsubmitted`, or `withdrawn`; a settled run takes
    /// nothing more.
    #[must_use]
    pub const fn can_follow(self, current: Option<Self>) -> bool {
        match current {
            None => true,
            Some(Self::Waiting) => self.settles_a_waiting_run(),
            Some(_) => false,
        }
    }

    /// Whether this result can end a run that waits in a queue.
    #[must_use]
    pub const fn settles_a_waiting_run(self) -> bool {
        matches!(self, Self::Submitted | Self::Unsubmitted | Self::Withdrawn)
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [
            Self::Submitted,
            Self::Waiting,
            Self::Unsubmitted,
            Self::Withdrawn,
            Self::Refused,
            Self::Skipped,
            Self::Failed,
            Self::Missed,
        ]
        .into_iter()
        .find(|result| result.as_str() == text)
    }
}

/// What started a run: a due time, or the person's Run now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunVia {
    Due,
    Now,
}

impl RunVia {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Due => "due",
            Self::Now => "now",
        }
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Due, Self::Now]
            .into_iter()
            .find(|via| via.as_str() == text)
    }
}

/// The result of a run with its reason and, for a directive that waits, the
/// queue entry that holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutcome {
    pub result: RunResult,
    pub reason: Option<String>,
    pub queue_entry: Option<String>,
}

/// A history row to add. `outcome` is `None` for a run that starts now and
/// ends later (an open run), and set for a row that is final at once, such as
/// a missed entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRun {
    pub schedule_id: ScheduleId,
    pub due: Timestamp,
    pub started: Timestamp,
    pub via: RunVia,
    pub outcome: Option<RunOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleRun {
    pub id: RunId,
    pub schedule_id: ScheduleId,
    pub due: Timestamp,
    pub started: Timestamp,
    pub via: RunVia,
    /// `None` while the run is open.
    pub outcome: Option<RunOutcome>,
}
