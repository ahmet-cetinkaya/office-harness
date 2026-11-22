// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Mutex;
use std::time::Duration;

use terminal_application::common::errors::TerminalRuntimeError;
use terminal_application::common::models::{
    Attachment, DeliveryStatus, LiveOutput, RunId, SpawnTerminalRequest,
};

use super::*;
use crate::handlers::ErrorKind;

const AGENT: &str = "7d0c4a52-1f3e-4b7a-9c11-0a1b2c3d4e5f";

#[derive(Default)]
struct FakeRuntime {
    written: Mutex<Vec<u8>>,
}

impl TerminalRuntime for FakeRuntime {
    fn spawn(&self, _request: &SpawnTerminalRequest) -> Result<(), TerminalRuntimeError> {
        Ok(())
    }

    fn attach(
        &self,
        _agent_id: AgentId,
        _saved_output: Option<&OutputKey>,
        _size: TerminalSize,
        _subscriber: Arc<dyn OutputSubscriber>,
    ) -> Result<Attachment, TerminalRuntimeError> {
        Ok(Attachment {
            snapshot: b"screen".to_vec(),
            subscription: Some(SubscriptionId(1)),
        })
    }

    fn detach(&self, _agent_id: AgentId, _subscription: SubscriptionId) {}

    fn write_input(&self, _agent_id: AgentId, bytes: &[u8]) -> Result<(), TerminalRuntimeError> {
        self.written.lock().expect("lock").extend_from_slice(bytes);
        Ok(())
    }

    fn paste(&self, _agent_id: AgentId, _text: &str) -> Result<(), TerminalRuntimeError> {
        Ok(())
    }

    fn resize(
        &self,
        _agent_id: AgentId,
        _size: TerminalSize,
    ) -> Result<bool, TerminalRuntimeError> {
        Ok(true)
    }

    fn stop(&self, _agent_id: AgentId, _grace: Duration) -> Result<(), TerminalRuntimeError> {
        Ok(())
    }

    fn current_run(&self, _agent_id: AgentId) -> Option<RunId> {
        None
    }

    fn quiet_for(&self, _agent_id: AgentId) -> Option<Duration> {
        None
    }

    fn is_running(&self, _agent_id: AgentId) -> bool {
        true
    }
}

struct Silent;

impl OutputSubscriber for Silent {
    fn deliver(&self, _output: LiveOutput) -> DeliveryStatus {
        DeliveryStatus::Accepted
    }
}

fn handlers(runtime: &Arc<FakeRuntime>) -> TerminalHandlers {
    let runtime: Arc<dyn TerminalRuntime> = runtime.clone();
    TerminalHandlers::new(&runtime)
}

#[test]
fn attach_returns_a_snapshot_frame() {
    let attached = handlers(&Arc::default())
        .attach(
            &AttachRequest {
                agent_id: AGENT.to_owned(),
                cols: 80,
                rows: 24,
            },
            None,
            Arc::new(Silent),
        )
        .expect("attach");
    assert_eq!(attached.snapshot_frame[0], 0x01);
    assert!(attached.snapshot_frame.ends_with(b"screen"));
    assert_eq!(attached.subscription, Some(SubscriptionId(1)));
}

#[test]
fn a_zero_size_names_the_field() {
    let error = handlers(&Arc::default())
        .resize(&ResizeRequest {
            agent_id: AGENT.to_owned(),
            cols: 0,
            rows: 24,
        })
        .expect_err("zero columns");
    assert_eq!(error.kind, ErrorKind::Invalid);
    assert!(error.message.contains("cols"));
}

#[test]
fn input_is_decoded_from_base64_and_passed_on_unchanged() {
    let runtime = Arc::new(FakeRuntime::default());
    handlers(&runtime)
        .input(&InputRequest {
            agent_id: AGENT.to_owned(),
            data: "G1tB".to_owned(),
        })
        .expect("input");
    assert_eq!(*runtime.written.lock().expect("lock"), b"\x1b[A");
}

#[test]
fn bad_base64_names_the_data_field() {
    let error = handlers(&Arc::default())
        .input(&InputRequest {
            agent_id: AGENT.to_owned(),
            data: "***".to_owned(),
        })
        .expect_err("bad base64");
    assert!(error.message.contains("data"));
}
