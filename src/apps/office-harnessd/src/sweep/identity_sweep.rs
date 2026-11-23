// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use agent_application::common::models::SessionProcess;
use terminal_application::common::interfaces::{
    EndOutcome, ProcessHandle, ProcessInspector, ReadError,
};
use terminal_application::common::models::{Mismatch, ProcessIdentity};

use super::leftover_result::{LeftReason, LeftoverResult};

/// How long to wait for a process that the forced end reached. A killed
/// process is gone at once; the wait only covers the OS finishing it.
pub const FORCED_END_WAIT: Duration = Duration::from_secs(2);

/// Sweeps one stored identity (D6): opens the process, ends it through that
/// handle only when every value still matches, first with a request and then
/// by force after `grace`. The match is read again just before each end.
pub fn sweep_identity(
    inspector: &dyn ProcessInspector,
    stored: &SessionProcess,
    grace: Duration,
) -> LeftoverResult {
    let expected = identity_of(stored);
    let Some(handle) = inspector.open(stored.pid) else {
        return LeftoverResult::Left(LeftReason::NotRunning);
    };
    // The second read is the one just before the request: the first only
    // decides whether the process is the session's at all.
    let matches_twice = check_match(handle.as_ref(), &expected)
        .and_then(|()| check_match(handle.as_ref(), &expected));
    if let Err(reason) = matches_twice {
        return LeftoverResult::Left(reason);
    }
    request_then_force(handle.as_ref(), &expected, grace)
}

fn identity_of(stored: &SessionProcess) -> ProcessIdentity {
    ProcessIdentity {
        pid: stored.pid,
        started_at: stored.started_at,
        command_hash: stored.command_hash,
        command_display: stored.command_display.clone(),
        boot: stored.boot.clone(),
    }
}

/// The reasons keep the order of the plan (D2): not readable and not
/// running come from the read, the rest from `ProcessIdentity::mismatch`.
fn check_match(handle: &dyn ProcessHandle, expected: &ProcessIdentity) -> Result<(), LeftReason> {
    match handle.identity() {
        Err(ReadError::NotRunning) => Err(LeftReason::NotRunning),
        Err(ReadError::NotReadable) => Err(LeftReason::NotReadable),
        Ok(live) => expected
            .mismatch(&live)
            .map_or(Ok(()), |mismatch| Err(LeftReason::Mismatch(mismatch))),
    }
}

fn request_then_force(
    handle: &dyn ProcessHandle,
    expected: &ProcessIdentity,
    grace: Duration,
) -> LeftoverResult {
    match handle.request_end() {
        Err(error) => return LeftoverResult::CouldNotEnd(error),
        Ok(EndOutcome::Gone) => return LeftoverResult::Ended,
        Ok(EndOutcome::Delivered) if handle.wait_for_exit(grace) => return LeftoverResult::Ended,
        // No polite request exists here: the check just before it is the
        // one that holds for the forced end.
        Ok(EndOutcome::Delivered | EndOutcome::Unsupported) => {}
    }
    if let Err(result) = recheck_before_force(handle, expected) {
        return result;
    }
    force(handle)
}

/// The process outlived the request, so it is read again. A process that is
/// gone, or that is another run now (an id handed out again on a system
/// without a handle), counts as ended and gets no force. A process that
/// cannot be read now is often one that is exiting (its command line goes
/// before its state does), so it gets a short wait before it counts as stuck.
fn recheck_before_force(
    handle: &dyn ProcessHandle,
    expected: &ProcessIdentity,
) -> Result<(), LeftoverResult> {
    match check_match(handle, expected) {
        Ok(()) => Ok(()),
        Err(LeftReason::NotRunning | LeftReason::Mismatch(Mismatch::OtherStartTime)) => {
            Err(LeftoverResult::Ended)
        }
        Err(LeftReason::NotReadable) if handle.wait_for_exit(FORCED_END_WAIT) => {
            Err(LeftoverResult::Ended)
        }
        Err(reason) => Err(LeftoverResult::CouldNotEnd(format!(
            "the process changed after the end request: {}",
            reason.text()
        ))),
    }
}

fn force(handle: &dyn ProcessHandle) -> LeftoverResult {
    match handle.force_end() {
        Err(error) => LeftoverResult::CouldNotEnd(error),
        Ok(EndOutcome::Gone) => LeftoverResult::Ended,
        Ok(EndOutcome::Delivered) if handle.wait_for_exit(FORCED_END_WAIT) => {
            LeftoverResult::EndedByForce
        }
        Ok(EndOutcome::Delivered) => {
            LeftoverResult::CouldNotEnd("still running after the forced end".to_owned())
        }
        Ok(EndOutcome::Unsupported) => {
            LeftoverResult::CouldNotEnd("the OS has no way to end the process".to_owned())
        }
    }
}

#[cfg(test)]
#[path = "identity_sweep.tests.rs"]
mod tests;
