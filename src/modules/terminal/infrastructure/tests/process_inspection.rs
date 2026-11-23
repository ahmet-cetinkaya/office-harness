// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! Runs the process inspection adapter against real child processes of the
//! test: the identity reads, the match, and the end through a handle.

use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use terminal_application::common::interfaces::{
    EndOutcome, ProcessHandle, ProcessInspector, ReadError,
};
use terminal_application::common::models::{Mismatch, ProcessIdentity};
use terminal_infrastructure::services::SystemProcessInspector;

const EXIT_WAIT: Duration = Duration::from_secs(10);
const SHORT_WAIT: Duration = Duration::from_millis(200);
/// Longer than a whole second, the start time unit on macOS, so two children
/// with the same command never start in the same tick.
const START_GAP: Duration = Duration::from_millis(1_100);
const READABLE_WAIT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// A child that ends with the test, even when the test fails.
struct Sleeper(Child);

impl Sleeper {
    fn start() -> Self {
        let child = Command::new(env!("CARGO_BIN_EXE_terminal-fixture"))
            .arg("sleep")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .expect("the fixture starts");
        let sleeper = Self(child);
        sleeper.wait_until_readable();
        sleeper
    }

    /// A new process has an empty command line for a moment while the kernel
    /// sets up its image, and an empty line is not readable. The sweep never
    /// meets this on a leftover; a test that reads a child at once does.
    fn wait_until_readable(&self) {
        let inspector = SystemProcessInspector::new();
        let deadline = Instant::now() + READABLE_WAIT;
        while inspector
            .open(self.pid())
            .is_none_or(|handle| handle.identity().is_err())
        {
            assert!(Instant::now() < deadline, "the child never became readable");
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn pid(&self) -> u32 {
        self.0.id()
    }

    fn is_running(&mut self) -> bool {
        self.0
            .try_wait()
            .expect("the child can be polled")
            .is_none()
    }

    /// The pidfd and the process handle report an exit a moment before the
    /// parent can reap it, so a test that asks the child waits for the reap.
    fn has_exited_soon(&mut self) -> bool {
        let deadline = Instant::now() + EXIT_WAIT;
        while self.is_running() {
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(POLL_INTERVAL);
        }
        true
    }

    /// Ends the child and reaps it, so its id is free for the OS to reuse.
    fn end_and_reap(&mut self) {
        let _ = self.0.kill();
        self.0.wait().expect("the child can be reaped");
    }
}

impl Drop for Sleeper {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn inspectors() -> Vec<SystemProcessInspector> {
    vec![
        SystemProcessInspector::new(),
        // Linux before 5.3 has no pidfd; the same checks run on that path.
        #[cfg(target_os = "linux")]
        SystemProcessInspector::without_pidfd(),
    ]
}

fn open(inspector: &SystemProcessInspector, child: &Sleeper) -> Box<dyn ProcessHandle> {
    inspector
        .open(child.pid())
        .expect("the child can be opened")
}

fn identity_of(handle: &dyn ProcessHandle) -> ProcessIdentity {
    handle.identity().expect("the child can be read")
}

#[test]
fn the_identity_of_a_child_reads_the_same_twice() {
    for inspector in inspectors() {
        let child = Sleeper::start();
        let handle = open(&inspector, &child);
        let first = identity_of(handle.as_ref());
        assert_eq!(first, identity_of(handle.as_ref()));
        assert_eq!(first.pid, child.pid());
        assert_eq!(Some(first.boot.clone()), inspector.boot());
    }
}

#[test]
fn an_identity_matches_itself_and_is_not_running_after_the_child_ends() {
    for inspector in inspectors() {
        let mut child = Sleeper::start();
        let handle = open(&inspector, &child);
        let stored = identity_of(handle.as_ref());
        assert_eq!(stored.mismatch(&identity_of(handle.as_ref())), None);

        child.end_and_reap();
        assert_eq!(handle.identity(), Err(ReadError::NotRunning));
    }
}

#[test]
fn the_same_command_started_later_has_another_start_time() {
    for inspector in inspectors() {
        let first = Sleeper::start();
        thread::sleep(START_GAP);
        let second = Sleeper::start();
        let first_identity = identity_of(open(&inspector, &first).as_ref());
        let second_identity = identity_of(open(&inspector, &second).as_ref());
        assert_eq!(first_identity.command_hash, second_identity.command_hash);
        assert_eq!(
            first_identity.mismatch(&second_identity),
            Some(Mismatch::OtherStartTime)
        );
    }
}

#[test]
fn a_start_time_from_the_handle_fits_the_signed_database_integer() {
    for inspector in inspectors() {
        let child = Sleeper::start();
        let started_at = identity_of(open(&inspector, &child).as_ref()).started_at;
        let stored = i64::try_from(started_at).expect("a start time fits a signed 64-bit integer");
        assert_eq!(u64::try_from(stored), Ok(started_at));
    }
}

#[cfg(windows)]
#[test]
fn a_windows_start_time_keeps_100_nanosecond_precision() {
    const FILETIME_UNITS_PER_SECOND: u64 = 10_000_000;
    let inspector = SystemProcessInspector::new();
    let child = Sleeper::start();
    let started_at = identity_of(open(&inspector, &child).as_ref()).started_at;
    assert_ne!(started_at % FILETIME_UNITS_PER_SECOND, 0);
}

#[test]
fn force_end_ends_only_that_process() {
    for inspector in inspectors() {
        let mut target = Sleeper::start();
        let mut sibling = Sleeper::start();
        let handle = open(&inspector, &target);
        assert_eq!(handle.force_end(), Ok(EndOutcome::Delivered));
        assert!(handle.wait_for_exit(EXIT_WAIT));
        assert!(target.has_exited_soon());
        assert!(sibling.is_running());
        assert!(open(&inspector, &sibling).identity().is_ok());
    }
}

#[test]
fn request_end_ends_only_that_process_where_a_request_exists() {
    for inspector in inspectors() {
        let mut target = Sleeper::start();
        let mut sibling = Sleeper::start();
        let handle = open(&inspector, &target);
        if cfg!(windows) {
            assert_eq!(handle.request_end(), Ok(EndOutcome::Unsupported));
            assert!(target.is_running());
        } else {
            assert_eq!(handle.request_end(), Ok(EndOutcome::Delivered));
            assert!(handle.wait_for_exit(EXIT_WAIT));
            assert!(target.has_exited_soon());
        }
        assert!(sibling.is_running());
    }
}

#[test]
fn a_running_process_does_not_exit_within_a_short_wait() {
    for inspector in inspectors() {
        let child = Sleeper::start();
        assert!(!open(&inspector, &child).wait_for_exit(SHORT_WAIT));
    }
}

#[test]
fn a_handle_on_a_reaped_child_reports_gone_and_signals_nothing_else() {
    for inspector in inspectors() {
        let mut target = Sleeper::start();
        let handle = open(&inspector, &target);
        target.end_and_reap();

        let mut later_sibling = Sleeper::start();
        assert_eq!(handle.request_end(), Ok(EndOutcome::Gone));
        assert_eq!(handle.force_end(), Ok(EndOutcome::Gone));
        assert!(later_sibling.is_running());
    }
}

#[test]
fn the_descendants_of_this_process_hold_its_child_and_not_itself() {
    let child = Sleeper::start();
    let found = SystemProcessInspector::new().descendants(std::process::id());
    assert!(found.contains(&child.pid()));
    assert!(!found.contains(&std::process::id()));
}

#[test]
fn the_machine_run_is_not_empty() {
    assert!(
        SystemProcessInspector::new()
            .boot()
            .is_some_and(|boot| !boot.is_empty())
    );
}
