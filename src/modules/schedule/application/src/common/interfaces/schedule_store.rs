// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::ScheduleId;
use schedule_domain::{Schedule, ScheduleCap};

use crate::common::errors::ScheduleApplicationError;
use crate::common::models::{NewRun, RunId, RunOutcome, ScheduleRun};

/// The kept schedules and their run history.
pub trait ScheduleStore: Send + Sync {
    /// # Errors
    /// `Storage`.
    fn list(&self) -> Result<Vec<Schedule>, ScheduleApplicationError>;

    /// # Errors
    /// `Storage`.
    fn get(&self, id: ScheduleId) -> Result<Option<Schedule>, ScheduleApplicationError>;

    /// Inserts a new schedule or replaces the one with the same id. In one
    /// transaction it checks the name without case against the other
    /// schedules, and for an insert it checks that the count stays within
    /// `cap`.
    ///
    /// # Errors
    /// `NameTaken`, `CapReached`, `Storage`.
    fn save(&self, schedule: &Schedule, cap: ScheduleCap) -> Result<(), ScheduleApplicationError>;

    /// Removes the schedule and its history; `false` when it was not there.
    ///
    /// # Errors
    /// `Storage`.
    fn delete(&self, id: ScheduleId) -> Result<bool, ScheduleApplicationError>;

    /// Adds a history row and, in the same transaction, drops the schedule's
    /// rows beyond the newest 20.
    ///
    /// # Errors
    /// `Storage`.
    fn record_run(&self, run: &NewRun) -> Result<RunId, ScheduleApplicationError>;

    /// # Errors
    /// `Storage`.
    fn get_run(&self, id: RunId) -> Result<Option<ScheduleRun>, ScheduleApplicationError>;

    /// Writes the outcome of a run; `false` when the row is gone.
    ///
    /// # Errors
    /// `Storage`.
    fn settle_run(&self, id: RunId, outcome: &RunOutcome)
    -> Result<bool, ScheduleApplicationError>;

    /// The schedule's last 20 runs, newest first.
    ///
    /// # Errors
    /// `Storage`.
    fn history(&self, id: ScheduleId) -> Result<Vec<ScheduleRun>, ScheduleApplicationError>;

    /// Settles every open run (no result yet) as `failed` with `reason`, at
    /// a server start; returns how many.
    ///
    /// # Errors
    /// `Storage`.
    fn fail_open_runs(&self, reason: &str) -> Result<usize, ScheduleApplicationError>;
}
