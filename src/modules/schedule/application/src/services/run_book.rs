// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::ScheduleId;

use crate::common::errors::ScheduleApplicationError;
use crate::common::interfaces::ScheduleStore;
use crate::common::models::{NewRun, RunId, RunOutcome, RunResult, ScheduleRun};

/// The reason of a run that is skipped because the previous one is not over.
pub const WAITING_REASON: &str = "previous run still waiting";
/// The reason of an open run at a server start.
pub const SETTLED_AT_START_REASON: &str = "the server stopped";

/// Opens and settles the runs of the history, and says when a new run must
/// skip because an earlier one is not settled.
#[derive(Clone)]
pub struct RunBook {
    store: Arc<dyn ScheduleStore>,
}

impl RunBook {
    #[must_use]
    pub fn new(store: Arc<dyn ScheduleStore>) -> Self {
        Self { store }
    }

    /// The reason a new run of the schedule is `skipped`, when an earlier run
    /// is still open or has the result `waiting`.
    ///
    /// # Errors
    /// `Storage`.
    pub fn skip_reason(
        &self,
        schedule_id: ScheduleId,
    ) -> Result<Option<&'static str>, ScheduleApplicationError> {
        let is_unsettled = |run: &ScheduleRun| {
            run.outcome
                .as_ref()
                .is_none_or(|outcome| outcome.result == RunResult::Waiting)
        };
        let history = self.store.history(schedule_id)?;
        Ok(history.iter().any(is_unsettled).then_some(WAITING_REASON))
    }

    /// Adds a history row (an open run, or a final row such as a missed entry).
    ///
    /// # Errors
    /// `Storage`.
    pub fn open(&self, run: &NewRun) -> Result<RunId, ScheduleApplicationError> {
        self.store.record_run(run)
    }

    /// Writes the result of a run. An open run takes any result; a run that
    /// waits takes `submitted`, `unsubmitted`, or `withdrawn`; a settled run
    /// takes nothing more.
    ///
    /// # Errors
    /// `NotFound` for an unknown run, `RunSettled` for a result that the run
    /// cannot take, `Storage`.
    pub fn settle(&self, id: RunId, outcome: &RunOutcome) -> Result<(), ScheduleApplicationError> {
        let run = self.store.get_run(id)?.ok_or_else(|| {
            ScheduleApplicationError::NotFound(format!("run {} does not exist", id.0))
        })?;
        let allowed = match run.outcome.as_ref().map(|current| current.result) {
            None => true,
            Some(RunResult::Waiting) => matches!(
                outcome.result,
                RunResult::Submitted | RunResult::Unsubmitted | RunResult::Withdrawn
            ),
            Some(_) => false,
        };
        if !allowed {
            return Err(ScheduleApplicationError::RunSettled(format!(
                "run {} cannot change to {}",
                id.0,
                outcome.result.as_str()
            )));
        }
        self.store.settle_run(id, outcome)?;
        Ok(())
    }

    /// Fails every open run at a server start (nothing is started again).
    ///
    /// # Errors
    /// `Storage`.
    pub fn fail_open_runs(&self) -> Result<usize, ScheduleApplicationError> {
        self.store.fail_open_runs(SETTLED_AT_START_REASON)
    }
}

#[cfg(test)]
#[path = "run_book.tests.rs"]
mod tests;
