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
    /// settled rows beyond the newest 20; an open or waiting row stays.
    ///
    /// # Errors
    /// `Storage`.
    fn record_run(&self, run: &NewRun) -> Result<RunId, ScheduleApplicationError>;

    /// # Errors
    /// `Storage`.
    fn get_run(&self, id: RunId) -> Result<Option<ScheduleRun>, ScheduleApplicationError>;

    /// Writes the outcome of a run in one step, when the run can still take it
    /// (`RunResult::can_follow`); `false` when the row is gone or already
    /// settled, so two writers never overwrite each other.
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

    /// Settles every open run (no result yet) and every run that waits as
    /// `failed` with `reason`, at a server start, when the queue that held
    /// them is gone; returns how many.
    ///
    /// # Errors
    /// `Storage`.
    fn fail_open_runs(&self, reason: &str) -> Result<usize, ScheduleApplicationError>;
}
