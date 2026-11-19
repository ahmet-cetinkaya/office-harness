// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]
// Some helpers serve only the Unix tests.
#![allow(dead_code)]

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use kernel::AgentId;
use terminal_application::common::interfaces::{
    OutputSubscriber, TerminalEventSink, TerminalRuntime,
};
use terminal_application::common::models::{
    DeliveryStatus, LiveOutput, RunId, SpawnTerminalRequest, SubscriptionId,
};
use terminal_domain::events::TerminalEvent;
use terminal_domain::value_objects::{OutputKey, OutputLimits, TerminalSize};
use terminal_infrastructure::services::{PortableTerminalRuntime, RuntimeSettings};

const WAIT_LIMIT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(20);
pub const QUIET_PERIOD: Duration = Duration::from_millis(150);

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let started = Instant::now();
    while !condition() {
        assert!(
            started.elapsed() < WAIT_LIMIT,
            "timed out waiting for {what}"
        );
        thread::sleep(POLL_INTERVAL);
    }
}

pub fn size(columns: u16, rows: u16) -> TerminalSize {
    TerminalSize::new(columns, rows).expect("valid size")
}

#[derive(Default)]
pub struct RecordingSink {
    events: Mutex<Vec<TerminalEvent>>,
    runs: Mutex<Vec<RunId>>,
    lines: Mutex<Vec<String>>,
}

impl RecordingSink {
    pub fn events(&self) -> Vec<TerminalEvent> {
        locked(&self.events).clone()
    }

    /// The run named by each event, in order.
    pub fn runs(&self) -> Vec<RunId> {
        locked(&self.runs).clone()
    }

    /// The lines in the order their reports arrived.
    pub fn lines(&self) -> Vec<String> {
        locked(&self.lines).clone()
    }

    pub fn has_exited(&self) -> bool {
        self.events()
            .iter()
            .any(|event| matches!(event, TerminalEvent::Exited { .. }))
    }
}

impl TerminalEventSink for RecordingSink {
    fn publish(&self, _agent_id: AgentId, run: RunId, event: TerminalEvent) {
        locked(&self.runs).push(run);
        locked(&self.events).push(event);
    }

    fn publish_line(&self, _agent_id: AgentId, _run: RunId, line: String) {
        locked(&self.lines).push(line);
    }
}

/// An attached client: the snapshot it got and everything delivered after.
pub struct Client {
    pub snapshot: Vec<u8>,
    pub subscription: Option<SubscriptionId>,
    live: Arc<LiveLog>,
}

#[derive(Default)]
struct LiveLog {
    items: Mutex<Vec<LiveOutput>>,
}

impl OutputSubscriber for LiveLog {
    fn deliver(&self, output: LiveOutput) -> DeliveryStatus {
        locked(&self.items).push(output);
        DeliveryStatus::Accepted
    }
}

impl Client {
    pub fn attach(
        runtime: &PortableTerminalRuntime,
        agent_id: AgentId,
        size: TerminalSize,
    ) -> Self {
        let live = Arc::new(LiveLog::default());
        let attachment = runtime
            .attach(
                agent_id,
                None,
                size,
                Arc::clone(&live) as Arc<dyn OutputSubscriber>,
            )
            .expect("attach");
        Self {
            snapshot: attachment.snapshot,
            subscription: attachment.subscription,
            live,
        }
    }

    /// The snapshot followed by the live output, as text.
    pub fn text(&self) -> String {
        let mut bytes = self.snapshot.clone();
        for item in locked(&self.live.items).iter() {
            if let LiveOutput::Bytes(chunk) = item {
                bytes.extend_from_slice(chunk);
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    pub fn dropped_notices(&self) -> usize {
        locked(&self.live.items)
            .iter()
            .filter(|item| **item == LiveOutput::OlderOutputDropped)
            .count()
    }
}

pub fn runtime_in(
    folder: &Path,
    limits: OutputLimits,
) -> (PortableTerminalRuntime, Arc<RecordingSink>) {
    let sink = Arc::new(RecordingSink::default());
    let runtime = PortableTerminalRuntime::new(
        RuntimeSettings {
            output_folder: folder.to_path_buf(),
            limits,
            quiet_period: QUIET_PERIOD,
        },
        Arc::clone(&sink) as Arc<dyn TerminalEventSink>,
    );
    (runtime, sink)
}

pub fn limits(max_lines: usize) -> OutputLimits {
    OutputLimits::with_lines(max_lines).expect("valid limits")
}

fn base_environment() -> Vec<(String, String)> {
    let path = std::env::var("PATH").unwrap_or_default();
    vec![
        ("PATH".to_owned(), path),
        ("TERM".to_owned(), "xterm".to_owned()),
    ]
}

/// Starts `program` with `arguments` for the agent and returns the key of its
/// output file.
pub fn spawn_program(
    runtime: &PortableTerminalRuntime,
    agent_id: AgentId,
    program: &str,
    arguments: &[&str],
) -> OutputKey {
    let output_key = OutputKey::parse(&agent_id.to_string()).expect("valid key");
    runtime
        .spawn(&SpawnTerminalRequest {
            agent_id,
            program: program.to_owned(),
            arguments: arguments
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect(),
            working_folder: std::env::temp_dir(),
            environment: base_environment(),
            size: size(80, 24),
            output_key: output_key.clone(),
        })
        .expect("spawn");
    output_key
}

pub fn spawn_fixture(
    runtime: &PortableTerminalRuntime,
    agent_id: AgentId,
    arguments: &[&str],
) -> OutputKey {
    spawn_program(
        runtime,
        agent_id,
        env!("CARGO_BIN_EXE_terminal-fixture"),
        arguments,
    )
}

/// True while the process runs. A zombie that nobody reaped counts as gone.
#[cfg(unix)]
pub fn process_is_alive(process_id: u32) -> bool {
    let answers_signal_zero = std::process::Command::new("kill")
        .args(["-0", &process_id.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    let is_zombie = std::fs::read_to_string(format!("/proc/{process_id}/stat"))
        .ok()
        .and_then(|stat| {
            stat.rsplit_once(')')
                .map(|(_, rest)| rest.trim_start().starts_with('Z'))
        })
        .unwrap_or(false);
    answers_signal_zero && !is_zombie
}

/// A file the fixture child waits for. The test creates it at the point it
/// chooses, so the child's silence ends on a handshake, not on a timer.
pub struct Release {
    path: std::path::PathBuf,
}

impl Release {
    pub fn in_folder(folder: &Path, name: &str) -> Self {
        Self {
            path: folder.join(name),
        }
    }

    /// The path as the fixture's argument.
    pub fn argument(&self) -> &str {
        self.path.to_str().expect("a UTF-8 temp path")
    }

    pub fn signal(&self) {
        std::fs::write(&self.path, b"").expect("release the child");
    }
}
