// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::models::SweepSummary;
use serde_json::{Value, json};

const COULD_NOT_END: &str = "could not end";
const NOT_READABLE: &str = "not readable";
const BOOT_TIME_CHANGED: &str = "boot time changed";
const NO_STORED_PROCESS: &str = "no stored process";
const HELPER_LIMIT: &str = "helper limit";
const NOT_SWEPT: &str = "not swept";

/// What the sweep found for the last session of an agent, when that session
/// was `lost` (0017/D8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UncleanStop {
    /// The sweep ended this many leftover processes.
    Ended(u32),
    /// A process of the session may still run, so the user is warned.
    MayRun(&'static str),
}

impl UncleanStop {
    /// `None` when the sweep left nothing that the user needs to know:
    /// every stored process was gone or was another process.
    ///
    /// When several reasons hold, the first of this list is the one given:
    /// could not end, not readable, boot time changed, no stored process,
    /// helper limit, not swept. The order goes from the surest sign of a
    /// running process to the weakest, so the text names the worst case.
    #[must_use]
    pub fn of(summary: &SweepSummary) -> Option<Self> {
        let may_run = [
            (summary.could_not_end > 0, COULD_NOT_END),
            (summary.not_readable > 0, NOT_READABLE),
            (summary.boot_time_changed > 0, BOOT_TIME_CHANGED),
            (summary.cli_rows == 0, NO_STORED_PROCESS),
            (summary.helpers_capped, HELPER_LIMIT),
            (summary.still_open > 0, NOT_SWEPT),
        ]
        .into_iter()
        .find_map(|(holds, reason)| holds.then_some(reason));
        match may_run {
            Some(reason) => Some(Self::MayRun(reason)),
            None => (summary.ended > 0).then_some(Self::Ended(summary.ended)),
        }
    }

    /// The `unclean_stop` value of a `resume.plan` entry.
    #[must_use]
    pub fn to_json(&self) -> Value {
        match self {
            Self::Ended(count) => json!({ "ended": count }),
            Self::MayRun(reason) => json!({ "may_run": reason }),
        }
    }

    /// The old `after_unclean_stop` boolean, kept for an older desktop app.
    #[must_use]
    pub fn warns(&self) -> bool {
        matches!(self, Self::MayRun(_))
    }
}

#[cfg(test)]
#[path = "unclean_stop.tests.rs"]
mod tests;
