// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

/// How many schedules the office may hold, on or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleCap(u32);

pub const MIN_SCHEDULE_CAP: u32 = 1;
pub const MAX_SCHEDULE_CAP: u32 = 200;
pub const DEFAULT_SCHEDULE_CAP: u32 = 50;

#[derive(Debug, Error, PartialEq, Eq)]
#[error("the schedule cap must be a whole number from {MIN_SCHEDULE_CAP} to {MAX_SCHEDULE_CAP}")]
pub struct ScheduleCapError;

impl ScheduleCap {
    /// # Errors
    /// `ScheduleCapError` for a value outside 1 to 200; the caller keeps the
    /// stored value.
    pub fn new(value: i64) -> Result<Self, ScheduleCapError> {
        u32::try_from(value)
            .ok()
            .filter(|cap| (MIN_SCHEDULE_CAP..=MAX_SCHEDULE_CAP).contains(cap))
            .map(Self)
            .ok_or(ScheduleCapError)
    }

    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }
}

impl Default for ScheduleCap {
    fn default() -> Self {
        Self(DEFAULT_SCHEDULE_CAP)
    }
}

#[cfg(test)]
#[path = "schedule_cap.tests.rs"]
mod tests;
