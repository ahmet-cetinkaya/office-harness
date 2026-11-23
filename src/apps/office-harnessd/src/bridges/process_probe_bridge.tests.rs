// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use terminal_application::common::errors::TerminalRuntimeError;
use terminal_application::common::interfaces::{
    EndOutcome, OutputSubscriber, ProcessHandle, ProcessInspector, TerminalRuntime,
};
use terminal_application::common::models::{
    Attachment, RunId, SpawnTerminalRequest, SubscriptionId,
};
use terminal_domain::value_objects::{OutputKey, TerminalSize};

use super::*;

const KNOWN_PID: u32 = 42;

struct FixedHandle(Result<ProcessIdentity, ReadError>);

impl ProcessHandle for FixedHandle {
    fn identity(&self) -> Result<ProcessIdentity, ReadError> {
        self.0.clone()
    }

    fn request_end(&self) -> Result<EndOutcome, String> {
        Ok(EndOutcome::Gone)
    }

    fn force_end(&self) -> Result<EndOutcome, String> {
        Ok(EndOutcome::Gone)
    }

    fn wait_for_exit(&self, _timeout: Duration) -> bool {
        true
    }
}

/// An inspector that knows one process id.
struct OneProcess(Result<ProcessIdentity, ReadError>);

impl ProcessInspector for OneProcess {
    fn open(&self, pid: u32) -> Option<Box<dyn ProcessHandle>> {
        (pid == KNOWN_PID).then(|| Box::new(FixedHandle(self.0.clone())) as Box<dyn ProcessHandle>)
    }

    fn boot(&self) -> Option<String> {
        None
    }

    fn descendants(&self, root: u32) -> Vec<u32> {
        vec![root + 1]
    }
}

/// A runtime whose only answer is the process id of the CLI.
struct ProcessIdOnly;

impl TerminalRuntime for ProcessIdOnly {
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

    fn write_input(&self, agent_id: AgentId, _bytes: &[u8]) -> Result<(), TerminalRuntimeError> {
        Err(TerminalRuntimeError::NotRunning(agent_id))
    }

    fn paste(&self, agent_id: AgentId, _text: &str) -> Result<(), TerminalRuntimeError> {
        Err(TerminalRuntimeError::NotRunning(agent_id))
    }

    fn resize(
        &self,
        _agent_id: AgentId,
        _size: TerminalSize,
    ) -> Result<bool, TerminalRuntimeError> {
        Ok(false)
    }

    fn stop(&self, _agent_id: AgentId, _grace: Duration) -> Result<(), TerminalRuntimeError> {
        Ok(())
    }

    fn is_running(&self, _agent_id: AgentId) -> bool {
        true
    }

    fn process_id(&self, _agent_id: AgentId) -> Option<u32> {
        Some(KNOWN_PID)
    }

    fn current_run(&self, _agent_id: AgentId) -> Option<RunId> {
        None
    }

    fn quiet_for(&self, _agent_id: AgentId) -> Option<Duration> {
        None
    }
}

fn probe(read: Result<ProcessIdentity, ReadError>) -> ProcessProbeBridge {
    ProcessProbeBridge::new(Arc::new(ProcessIdOnly), Arc::new(OneProcess(read)))
}

fn identity() -> ProcessIdentity {
    ProcessIdentity::new(KNOWN_PID, 9, "claude --flag", "boot".to_owned())
}

#[test]
fn a_read_carries_every_value_of_the_identity() {
    let expected = identity();

    let read = probe(Ok(expected.clone())).read(KNOWN_PID).expect("read");

    assert_eq!(read.pid, expected.pid);
    assert_eq!(read.started_at, expected.started_at);
    assert_eq!(read.command_hash, expected.command_hash);
    assert_eq!(read.command_display, expected.command_display);
    assert_eq!(read.boot, expected.boot);
}

#[test]
fn a_process_that_cannot_be_opened_reads_as_not_running() {
    let result = probe(Ok(identity())).read(KNOWN_PID + 1);

    assert_eq!(result, Err(ProbeError::NotRunning));
}

#[test]
fn the_reason_of_a_failed_read_is_kept() {
    assert_eq!(
        probe(Err(ReadError::NotReadable)).read(KNOWN_PID),
        Err(ProbeError::NotReadable)
    );
    assert_eq!(
        probe(Err(ReadError::NotRunning)).read(KNOWN_PID),
        Err(ProbeError::NotRunning)
    );
}

#[test]
fn the_cli_process_id_and_the_descendants_come_from_the_terminal_module() {
    let probe = probe(Ok(identity()));

    assert_eq!(probe.cli_process_id(AgentId::generate()), Some(KNOWN_PID));
    assert_eq!(probe.descendants(KNOWN_PID), vec![KNOWN_PID + 1]);
}
