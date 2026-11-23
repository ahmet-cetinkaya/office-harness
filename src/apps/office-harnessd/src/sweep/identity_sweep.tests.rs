// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, PoisonError};

use super::*;

const GRACE: Duration = Duration::from_millis(10);
const PID: u32 = 4242;
const STARTED_AT: u64 = 1_000;
const BOOT_ID: &str = "boot-a";

fn stored() -> SessionProcess {
    SessionProcess {
        pid: PID,
        started_at: STARTED_AT,
        command_hash: [7; 32],
        command_display: "sleep 600".to_owned(),
        boot: BOOT_ID.to_owned(),
    }
}

fn live(started_at: u64, command_hash: [u8; 32], boot: &str) -> ProcessIdentity {
    ProcessIdentity {
        pid: PID,
        started_at,
        command_hash,
        command_display: "sleep 600".to_owned(),
        boot: boot.to_owned(),
    }
}

fn matching() -> ProcessIdentity {
    live(STARTED_AT, [7; 32], BOOT_ID)
}

/// What a fake handle did, for the assertions.
#[derive(Default)]
struct Calls {
    reads: usize,
    requests: usize,
    forces: usize,
}

/// A handle with a script: the identities it reads in turn (the last one
/// repeats), the answers to the end calls, and whether the process exits
/// within each wait.
struct ScriptedHandle {
    reads: Mutex<VecDeque<Result<ProcessIdentity, ReadError>>>,
    request: Result<EndOutcome, String>,
    force: Result<EndOutcome, String>,
    exits: Mutex<VecDeque<bool>>,
    calls: Arc<Mutex<Calls>>,
}

impl ScriptedHandle {
    fn new(reads: Vec<Result<ProcessIdentity, ReadError>>) -> (Self, Arc<Mutex<Calls>>) {
        let calls = Arc::new(Mutex::new(Calls::default()));
        let handle = Self {
            reads: Mutex::new(reads.into()),
            request: Ok(EndOutcome::Delivered),
            force: Ok(EndOutcome::Delivered),
            exits: Mutex::new(VecDeque::new()),
            calls: Arc::clone(&calls),
        };
        (handle, calls)
    }

    fn with_exits(self, exits: Vec<bool>) -> Self {
        *self.exits.lock().unwrap_or_else(PoisonError::into_inner) = exits.into();
        self
    }
}

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl ProcessHandle for ScriptedHandle {
    fn identity(&self) -> Result<ProcessIdentity, ReadError> {
        locked(&self.calls).reads += 1;
        let mut reads = locked(&self.reads);
        if reads.len() > 1 {
            return reads.pop_front().expect("a scripted read");
        }
        reads.front().cloned().expect("a scripted read")
    }

    fn request_end(&self) -> Result<EndOutcome, String> {
        locked(&self.calls).requests += 1;
        self.request.clone()
    }

    fn force_end(&self) -> Result<EndOutcome, String> {
        locked(&self.calls).forces += 1;
        self.force.clone()
    }

    fn wait_for_exit(&self, _timeout: Duration) -> bool {
        locked(&self.exits).pop_front().unwrap_or(false)
    }
}

struct OneHandle(Mutex<Option<Box<dyn ProcessHandle>>>);

impl OneHandle {
    fn of(handle: ScriptedHandle) -> Self {
        Self(Mutex::new(Some(Box::new(handle))))
    }

    fn none() -> Self {
        Self(Mutex::new(None))
    }
}

impl ProcessInspector for OneHandle {
    fn open(&self, _pid: u32) -> Option<Box<dyn ProcessHandle>> {
        locked(&self.0).take()
    }

    fn boot(&self) -> Option<String> {
        None
    }

    fn descendants(&self, _root: u32) -> Vec<u32> {
        Vec::new()
    }
}

fn sweep_with(handle: ScriptedHandle) -> LeftoverResult {
    sweep_identity(&OneHandle::of(handle), &stored(), GRACE)
}

#[test]
fn a_process_that_cannot_be_opened_is_not_running() {
    let result = sweep_identity(&OneHandle::none(), &stored(), GRACE);

    assert_eq!(result, LeftoverResult::Left(LeftReason::NotRunning));
}

#[test]
fn the_reasons_follow_the_order_of_the_plan() {
    let other_hash = [8; 32];
    let cases = [
        (Err(ReadError::NotReadable), LeftReason::NotReadable),
        (Err(ReadError::NotRunning), LeftReason::NotRunning),
        (
            Ok(live(STARTED_AT, [7; 32], "boot-b")),
            LeftReason::Mismatch(Mismatch::MachineRestarted),
        ),
        (
            Ok(live(STARTED_AT + 1, other_hash, "boot-b")),
            LeftReason::Mismatch(Mismatch::MachineRestarted),
        ),
        (
            Ok(live(STARTED_AT + 1, other_hash, BOOT_ID)),
            LeftReason::Mismatch(Mismatch::OtherStartTime),
        ),
        (
            Ok(live(STARTED_AT, other_hash, BOOT_ID)),
            LeftReason::Mismatch(Mismatch::OtherCommandLine),
        ),
    ];
    for (read, expected) in cases {
        let (handle, calls) = ScriptedHandle::new(vec![read.clone()]);

        let result = sweep_with(handle);

        assert_eq!(result, LeftoverResult::Left(expected), "{read:?}");
        assert_eq!(locked(&calls).requests, 0, "{read:?}");
        assert_eq!(locked(&calls).forces, 0, "{read:?}");
    }
}

#[test]
fn a_boot_time_that_differs_by_three_seconds_is_left_as_changed() {
    let mut process = stored();
    process.boot = "1000".to_owned();
    let (handle, calls) = ScriptedHandle::new(vec![Ok(live(STARTED_AT, [7; 32], "1003"))]);

    let result = sweep_identity(&OneHandle::of(handle), &process, GRACE);

    assert_eq!(
        result,
        LeftoverResult::Left(LeftReason::Mismatch(Mismatch::BootTimeChanged))
    );
    assert_eq!(locked(&calls).requests, 0);
}

#[test]
fn another_start_time_wins_over_a_changed_boot_time() {
    let mut process = stored();
    process.boot = "1000".to_owned();
    let (handle, _calls) = ScriptedHandle::new(vec![Ok(live(STARTED_AT + 1, [7; 32], "1003"))]);

    let result = sweep_identity(&OneHandle::of(handle), &process, GRACE);

    assert_eq!(
        result,
        LeftoverResult::Left(LeftReason::Mismatch(Mismatch::OtherStartTime))
    );
}

#[test]
fn a_process_that_ends_on_the_request_is_ended_and_not_forced() {
    let (handle, calls) = ScriptedHandle::new(vec![Ok(matching())]);

    let result = sweep_with(handle.with_exits(vec![true]));

    assert_eq!(result, LeftoverResult::Ended);
    assert_eq!(locked(&calls).requests, 1);
    assert_eq!(locked(&calls).forces, 0);
}

#[test]
fn the_match_is_read_again_just_before_the_request() {
    let (handle, calls) = ScriptedHandle::new(vec![Ok(matching()), Err(ReadError::NotRunning)]);

    let result = sweep_with(handle);

    assert_eq!(result, LeftoverResult::Left(LeftReason::NotRunning));
    assert_eq!(locked(&calls).reads, 2);
    assert_eq!(locked(&calls).requests, 0);
}

#[test]
fn a_process_that_ignores_the_request_is_ended_by_force() {
    let (handle, calls) = ScriptedHandle::new(vec![Ok(matching())]);

    let result = sweep_with(handle.with_exits(vec![false, true]));

    assert_eq!(result, LeftoverResult::EndedByForce);
    assert_eq!(locked(&calls).requests, 1);
    assert_eq!(locked(&calls).forces, 1);
}

#[test]
fn a_handle_that_reports_gone_gets_no_second_end_call() {
    let (mut handle, calls) = ScriptedHandle::new(vec![Ok(matching())]);
    handle.request = Ok(EndOutcome::Gone);

    let result = sweep_with(handle);

    assert_eq!(result, LeftoverResult::Ended);
    assert_eq!(locked(&calls).forces, 0);
}

#[test]
fn a_process_that_goes_during_the_grace_period_is_not_forced() {
    let (handle, calls) = ScriptedHandle::new(vec![
        Ok(matching()),
        Ok(matching()),
        Err(ReadError::NotRunning),
    ]);

    let result = sweep_with(handle.with_exits(vec![false]));

    assert_eq!(result, LeftoverResult::Ended);
    assert_eq!(locked(&calls).forces, 0);
}

#[test]
fn an_id_handed_out_again_during_the_grace_period_is_not_forced() {
    let (handle, calls) = ScriptedHandle::new(vec![
        Ok(matching()),
        Ok(matching()),
        Ok(live(STARTED_AT + 5, [9; 32], BOOT_ID)),
    ]);

    let result = sweep_with(handle.with_exits(vec![false]));

    assert_eq!(result, LeftoverResult::Ended);
    assert_eq!(locked(&calls).forces, 0);
}

#[test]
fn a_process_that_changed_its_command_after_the_request_is_not_forced() {
    let (handle, calls) = ScriptedHandle::new(vec![
        Ok(matching()),
        Ok(matching()),
        Ok(live(STARTED_AT, [9; 32], BOOT_ID)),
    ]);

    let result = sweep_with(handle.with_exits(vec![false]));

    assert_eq!(
        result,
        LeftoverResult::CouldNotEnd(
            "the process changed after the end request: other command line".to_owned()
        )
    );
    assert_eq!(locked(&calls).forces, 0);
}

#[test]
fn a_refused_request_could_not_end_with_the_os_text() {
    let (mut handle, calls) = ScriptedHandle::new(vec![Ok(matching())]);
    handle.request = Err("Operation not permitted (os error 1)".to_owned());

    let result = sweep_with(handle);

    assert_eq!(
        result,
        LeftoverResult::CouldNotEnd("Operation not permitted (os error 1)".to_owned())
    );
    assert_eq!(locked(&calls).forces, 0);
}

#[test]
fn a_process_that_survives_the_force_could_not_end() {
    let (handle, _calls) = ScriptedHandle::new(vec![Ok(matching())]);

    let result = sweep_with(handle.with_exits(vec![false, false]));

    assert_eq!(
        result,
        LeftoverResult::CouldNotEnd("still running after the forced end".to_owned())
    );
}

#[test]
fn a_refused_force_could_not_end_with_the_os_text() {
    let (mut handle, _calls) = ScriptedHandle::new(vec![Ok(matching())]);
    handle.force = Err("Access is denied. (os error 5)".to_owned());

    let result = sweep_with(handle.with_exits(vec![false]));

    assert_eq!(
        result,
        LeftoverResult::CouldNotEnd("Access is denied. (os error 5)".to_owned())
    );
}

#[test]
fn an_os_without_a_polite_request_ends_by_force_at_once() {
    let (mut handle, calls) = ScriptedHandle::new(vec![Ok(matching())]);
    handle.request = Ok(EndOutcome::Unsupported);

    let result = sweep_with(handle.with_exits(vec![true]));

    assert_eq!(result, LeftoverResult::EndedByForce);
    assert_eq!(locked(&calls).forces, 1);
}

#[test]
fn a_process_that_cannot_be_read_while_it_exits_after_the_request_is_ended() {
    let (handle, calls) = ScriptedHandle::new(vec![
        Ok(matching()),
        Ok(matching()),
        Err(ReadError::NotReadable),
    ]);

    let result = sweep_with(handle.with_exits(vec![false, true]));

    assert_eq!(result, LeftoverResult::Ended);
    assert_eq!(locked(&calls).forces, 0);
}

#[test]
fn a_process_that_stays_unreadable_after_the_request_is_not_forced_and_could_not_end() {
    let (handle, calls) = ScriptedHandle::new(vec![
        Ok(matching()),
        Ok(matching()),
        Err(ReadError::NotReadable),
    ]);

    let result = sweep_with(handle.with_exits(vec![false, false]));

    assert_eq!(
        result,
        LeftoverResult::CouldNotEnd(
            "the process changed after the end request: not readable".to_owned()
        )
    );
    assert_eq!(locked(&calls).forces, 0);
}
