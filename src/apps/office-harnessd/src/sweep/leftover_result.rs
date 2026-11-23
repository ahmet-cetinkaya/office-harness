// SPDX-License-Identifier: AGPL-3.0-or-later

use audit_domain::ActionResult;
use terminal_application::common::models::Mismatch;

/// Why a stored process was not ended and left alone (FR-005).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeftReason {
    NotRunning,
    NotReadable,
    Mismatch(Mismatch),
}

impl LeftReason {
    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            Self::NotRunning => "not running",
            Self::NotReadable => "not readable",
            Self::Mismatch(Mismatch::MachineRestarted) => "machine restarted",
            Self::Mismatch(Mismatch::OtherStartTime) => "other start time",
            Self::Mismatch(Mismatch::OtherCommandLine) => "other command line",
            Self::Mismatch(Mismatch::BootTimeChanged) => "boot time changed",
        }
    }
}

/// What the sweep did with one stored process identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeftoverResult {
    Ended,
    EndedByForce,
    /// Still running after the forced end, or the OS refused; the text is the
    /// reason.
    CouldNotEnd(String),
    Left(LeftReason),
}

impl LeftoverResult {
    /// The result and its reason as the action record detail writes them.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Self::Ended => "ended".to_owned(),
            Self::EndedByForce => "ended by force".to_owned(),
            Self::CouldNotEnd(reason) => format!("could not end: {reason}"),
            Self::Left(reason) => format!("left: {}", reason.text()),
        }
    }

    /// A `could not end` is the one result that fails the action.
    #[must_use]
    pub fn action_result(&self) -> ActionResult {
        match self {
            Self::CouldNotEnd(reason) => ActionResult::Failed(reason.clone()),
            Self::Ended | Self::EndedByForce | Self::Left(_) => ActionResult::Ok,
        }
    }
}
