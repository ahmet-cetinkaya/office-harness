// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

use std::sync::Mutex;
use std::time::Duration;

use terminal_application::common::interfaces::OutputSubscriber;
use terminal_application::common::models::{Attachment, SpawnTerminalRequest, SubscriptionId};
use terminal_domain::value_objects::{OutputKey, TerminalSize};

use super::*;

/// A runtime that records what is pasted and written, and answers `paste`
/// with a chosen outcome.
struct RecordingRuntime {
    current: Option<RunId>,
    paste_error: Option<fn(AgentId) -> TerminalRuntimeError>,
    pasted: Mutex<Vec<String>>,
    written: Mutex<Vec<Vec<u8>>>,
}

impl RecordingRuntime {
    fn new(
        current: Option<RunId>,
        paste_error: Option<fn(AgentId) -> TerminalRuntimeError>,
    ) -> Self {
        Self {
            current,
            paste_error,
            pasted: Mutex::default(),
            written: Mutex::default(),
        }
    }
}

impl TerminalRuntime for RecordingRuntime {
    fn spawn(&self, _request: &SpawnTerminalRequest) -> Result<(), TerminalRuntimeError> {
        Ok(())
    }

    fn attach(
        &self,
        agent_id: AgentId,
        _saved_output: Option<&OutputKey>,
        _size: TerminalSize,
        _subscriber: Arc<dyn OutputSubscriber>,
    ) -> Result<Attachment, TerminalRuntimeError> {
        Err(TerminalRuntimeError::NoOutput(agent_id))
    }

    fn detach(&self, _agent_id: AgentId, _subscription: SubscriptionId) {}

    fn write_input(&self, _agent_id: AgentId, bytes: &[u8]) -> Result<(), TerminalRuntimeError> {
        self.written.lock().expect("lock").push(bytes.to_vec());
        Ok(())
    }

    fn paste(&self, agent_id: AgentId, text: &str) -> Result<(), TerminalRuntimeError> {
        self.pasted.lock().expect("lock").push(text.to_owned());
        self.paste_error
            .map_or(Ok(()), |error| Err(error(agent_id)))
    }

    fn resize(&self, agent_id: AgentId, _size: TerminalSize) -> Result<bool, TerminalRuntimeError> {
        Err(TerminalRuntimeError::NotRunning(agent_id))
    }

    fn stop(&self, agent_id: AgentId, _grace: Duration) -> Result<(), TerminalRuntimeError> {
        Err(TerminalRuntimeError::NotRunning(agent_id))
    }

    fn is_running(&self, _agent_id: AgentId) -> bool {
        true
    }

    fn current_run(&self, _agent_id: AgentId) -> Option<RunId> {
        self.current
    }
}

fn not_running(agent_id: AgentId) -> TerminalRuntimeError {
    TerminalRuntimeError::NotRunning(agent_id)
}

fn broken(_agent_id: AgentId) -> TerminalRuntimeError {
    TerminalRuntimeError::Io("closed".to_owned())
}

#[test]
fn the_prompt_is_pasted_and_never_submitted() {
    let run = RunId(1);
    let runtime = RecordingRuntime::new(Some(run), None);

    assert!(paste_into_run(&runtime, AgentId::generate(), run, "do it").expect("paste"));

    assert_eq!(*runtime.pasted.lock().expect("lock"), ["do it"]);
    assert!(runtime.written.lock().expect("lock").is_empty());
}

#[test]
fn a_prompt_of_an_earlier_run_is_not_pasted_into_a_newer_one() {
    let runtime = RecordingRuntime::new(Some(RunId(2)), None);

    assert!(!paste_into_run(&runtime, AgentId::generate(), RunId(1), "old").expect("skipped"));

    assert!(runtime.pasted.lock().expect("lock").is_empty());
}

#[test]
fn a_child_that_ended_is_not_a_failure_but_another_error_is() {
    let run = RunId(1);
    let ended = RecordingRuntime::new(Some(run), Some(not_running));
    let failing = RecordingRuntime::new(Some(run), Some(broken));

    assert!(matches!(
        paste_into_run(&ended, AgentId::generate(), run, "x"),
        Ok(false)
    ));
    assert!(matches!(
        paste_into_run(&failing, AgentId::generate(), run, "x"),
        Err(TerminalRuntimeError::Io(_))
    ));
}
