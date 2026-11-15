// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use kernel::AgentId;
use portable_pty::{Child, MasterPty};
use terminal_application::common::errors::TerminalRuntimeError;
use terminal_application::common::interfaces::{OutputSubscriber, TerminalEventSink};
use terminal_application::common::models::{
    Attachment, DeliveryStatus, LiveOutput, RunId, SubscriptionId,
};
use terminal_domain::events::TerminalEvent;
use terminal_domain::value_objects::TerminalSize;

use crate::services::output_store::OutputStore;
use crate::services::process_tree;
use crate::services::pty_host::{SpawnedPty, pty_size};
use crate::services::screen_emulator::ScreenEmulator;

const READ_BUFFER_BYTES: usize = 8 * 1024;
/// On Windows the PTY may never report the end of its output after the child
/// exits, so the exit waits this long for the reader and then goes on.
const DRAIN_GRACE: Duration = Duration::from_millis(300);
/// How long a forced end may take before the stop counts as failed.
const FORCE_GRACE: Duration = Duration::from_secs(3);
const MIN_MONITOR_TICK: Duration = Duration::from_millis(10);
const MAX_MONITOR_TICK: Duration = Duration::from_millis(250);
const CTRL_C: u8 = 0x03;
const PASTE_START: &[u8] = b"\x1b[200~";
const PASTE_END: &[u8] = b"\x1b[201~";
/// Reported when the wait on the child itself fails.
const UNKNOWN_EXIT_CODE: u32 = u32::MAX;

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn io_failure(error: impl std::fmt::Display) -> TerminalRuntimeError {
    TerminalRuntimeError::Io(error.to_string())
}

struct Subscription {
    id: SubscriptionId,
    subscriber: Arc<dyn OutputSubscriber>,
}

/// Everything that output, attach, and resize change together. One lock keeps
/// a snapshot and the live subscription atomic: output cannot land between
/// them.
struct SessionCore {
    emulator: ScreenEmulator,
    store: Option<OutputStore>,
    subscribers: Vec<Subscription>,
    size: TerminalSize,
    output_seen: bool,
    last_output: Instant,
    quiet_reported: bool,
}

impl SessionCore {
    fn broadcast(&mut self, output: &LiveOutput) {
        self.subscribers
            .retain(|entry| entry.subscriber.deliver(output.clone()) == DeliveryStatus::Accepted);
    }

    /// Writes to the output file. Returns whether the oldest output was
    /// dropped, and an event when the file can no longer be written.
    fn record(&mut self, bytes: &[u8]) -> (bool, Option<TerminalEvent>) {
        let Some(store) = self.store.as_mut() else {
            return (false, None);
        };
        if let Ok(outcome) = store.append(bytes) {
            return (outcome.dropped_oldest, None);
        }
        self.store = None;
        (false, Some(TerminalEvent::OutputStorageFailed))
    }
}

#[derive(Default)]
struct Lifecycle {
    /// Set the moment the wait on the child returns, before the output drain.
    child_exited: bool,
    reader_finished: bool,
    exit_code: Option<u32>,
}

/// What a session needs from outside to start.
pub struct SessionParts {
    pub agent_id: AgentId,
    pub pty: SpawnedPty,
    pub emulator: ScreenEmulator,
    pub store: OutputStore,
    pub size: TerminalSize,
    pub quiet_period: Duration,
    pub sink: Arc<dyn TerminalEventSink>,
    pub run: RunId,
}

/// One running child in a PTY with its screen, its output file, and its
/// attached clients. Three threads serve it: one reads output, one waits for
/// the exit, and one watches for the quiet period.
pub struct TerminalSession {
    agent_id: AgentId,
    run: RunId,
    process_id: Option<u32>,
    core: Mutex<SessionCore>,
    writer: Mutex<Box<dyn Write + Send>>,
    master: Mutex<Option<Box<dyn MasterPty + Send>>>,
    lifecycle: Mutex<Lifecycle>,
    lifecycle_changed: Condvar,
    sink: Arc<dyn TerminalEventSink>,
    next_subscription: AtomicU64,
}

impl TerminalSession {
    #[must_use]
    pub fn run(&self) -> RunId {
        self.run
    }

    #[must_use]
    pub fn start(parts: SessionParts) -> Arc<Self> {
        let SessionParts {
            agent_id,
            pty,
            emulator,
            store,
            size,
            quiet_period,
            sink,
            run,
        } = parts;
        let session = Arc::new(Self {
            agent_id,
            run,
            process_id: pty.child.process_id(),
            core: Mutex::new(SessionCore {
                emulator,
                store: Some(store),
                subscribers: Vec::new(),
                size,
                output_seen: false,
                last_output: Instant::now(),
                quiet_reported: false,
            }),
            writer: Mutex::new(pty.writer),
            master: Mutex::new(Some(pty.master)),
            lifecycle: Mutex::new(Lifecycle::default()),
            lifecycle_changed: Condvar::new(),
            sink,
            next_subscription: AtomicU64::new(1),
        });
        let reading = Arc::clone(&session);
        thread::spawn(move || reading.run_reader(pty.reader));
        let waiting = Arc::clone(&session);
        thread::spawn(move || waiting.run_waiter(pty.child));
        let watching = Arc::clone(&session);
        thread::spawn(move || watching.run_quiet_monitor(quiet_period));
        session
    }

    fn child_has_exited(&self) -> bool {
        locked(&self.lifecycle).child_exited
    }

    pub fn has_exited(&self) -> bool {
        locked(&self.lifecycle).exit_code.is_some()
    }

    /// # Errors
    /// `NotRunning` when the child is gone, or `Io` when the resize fails.
    pub fn attach(
        &self,
        size: TerminalSize,
        subscriber: Arc<dyn OutputSubscriber>,
    ) -> Result<Attachment, TerminalRuntimeError> {
        let mut core = locked(&self.core);
        self.resize_locked(&mut core, size)?;
        let snapshot = core.emulator.snapshot();
        let id = SubscriptionId(self.next_subscription.fetch_add(1, Ordering::Relaxed));
        core.subscribers.push(Subscription { id, subscriber });
        Ok(Attachment {
            snapshot,
            subscription: Some(id),
        })
    }

    pub fn detach(&self, subscription: SubscriptionId) {
        locked(&self.core)
            .subscribers
            .retain(|entry| entry.id != subscription);
    }

    /// # Errors
    /// `NotRunning` when the child is gone, or `Io` when the write fails.
    pub fn write_input(&self, bytes: &[u8]) -> Result<(), TerminalRuntimeError> {
        if self.has_exited() {
            return Err(TerminalRuntimeError::NotRunning(self.agent_id));
        }
        let mut writer = locked(&self.writer);
        writer
            .write_all(bytes)
            .and_then(|()| writer.flush())
            .map_err(io_failure)
    }

    /// # Errors
    /// Same as `write_input`.
    pub fn paste(&self, text: &str) -> Result<(), TerminalRuntimeError> {
        if !locked(&self.core).emulator.bracketed_paste() {
            return self.write_input(text.as_bytes());
        }
        let mut bytes = PASTE_START.to_vec();
        bytes.extend_from_slice(text.as_bytes());
        bytes.extend_from_slice(PASTE_END);
        self.write_input(&bytes)
    }

    /// Returns whether the size changed; the last resize wins.
    ///
    /// # Errors
    /// `NotRunning` when the child is gone, or `Io` when the resize fails.
    pub fn resize(&self, size: TerminalSize) -> Result<bool, TerminalRuntimeError> {
        let mut core = locked(&self.core);
        self.resize_locked(&mut core, size)
    }

    /// Asks the child's tree to end, waits `grace`, and then forces it.
    ///
    /// # Errors
    /// `NotRunning` when the child is gone, or `StopFailed` when it is still
    /// alive after the forced end.
    pub fn stop(&self, grace: Duration) -> Result<(), TerminalRuntimeError> {
        let not_running = TerminalRuntimeError::NotRunning(self.agent_id);
        let Some(root) = self.process_id else {
            return Err(not_running);
        };
        if self.has_exited() {
            return Err(not_running);
        }
        let descendants = process_tree::descendants(root);
        process_tree::request_end(root);
        if cfg!(windows) {
            // The interrupt key is the only end request that reaches a
            // console child. A write that fails means the child is gone.
            let _ = self.write_input(&[CTRL_C]);
        }
        self.wait_for_exit(grace);
        // Forced even after a clean exit: a grandchild may have outlived its
        // parent, and the group ends only when every member is gone.
        // On Windows the end goes by process id, and a freed id may belong to
        // an unrelated process by now, so it runs only while the root lives.
        // Unix ends the group, which no other process can take over.
        if !cfg!(windows) || !self.child_has_exited() {
            process_tree::force_end(root, &descendants);
        }
        if self.wait_for_exit(FORCE_GRACE) {
            Ok(())
        } else {
            Err(TerminalRuntimeError::StopFailed)
        }
    }

    fn resize_locked(
        &self,
        core: &mut SessionCore,
        size: TerminalSize,
    ) -> Result<bool, TerminalRuntimeError> {
        if core.size == size {
            return Ok(false);
        }
        let master = locked(&self.master);
        let master = master
            .as_ref()
            .ok_or(TerminalRuntimeError::NotRunning(self.agent_id))?;
        master.resize(pty_size(size)).map_err(io_failure)?;
        core.emulator.resize(size);
        core.size = size;
        Ok(true)
    }

    fn run_reader(&self, mut reader: Box<dyn Read + Send>) {
        let mut buffer = [0_u8; READ_BUFFER_BYTES];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(count) => self.handle_output(&buffer[..count]),
            }
        }
        locked(&self.lifecycle).reader_finished = true;
        self.lifecycle_changed.notify_all();
    }

    fn handle_output(&self, bytes: &[u8]) {
        let mut events = Vec::new();
        let replies = {
            let mut core = locked(&self.core);
            let replies = core.emulator.process(bytes);
            let (dropped_oldest, storage_event) = core.record(bytes);
            events.extend(storage_event);
            core.broadcast(&LiveOutput::Bytes(bytes.to_vec()));
            if dropped_oldest {
                core.broadcast(&LiveOutput::OlderOutputDropped);
            }
            if !core.output_seen {
                core.output_seen = true;
                events.push(TerminalEvent::FirstOutput);
            }
            if core.quiet_reported {
                events.push(TerminalEvent::OutputResumed);
            }
            core.last_output = Instant::now();
            core.quiet_reported = false;
            replies
        };
        if !replies.is_empty() {
            // A failed answer means the child is gone; the waiter reports it.
            let _ = self.write_input(&replies);
        }
        for event in events {
            self.sink.publish(self.agent_id, self.run, event);
        }
    }

    fn run_waiter(&self, mut child: Box<dyn Child + Send + Sync>) {
        let exit_code = child
            .wait()
            .map_or(UNKNOWN_EXIT_CODE, |status| status.exit_code());
        locked(&self.lifecycle).child_exited = true;
        self.wait_for_reader(DRAIN_GRACE);
        // Dropping the master closes the PTY, which ends a reader that a
        // lingering grandchild would otherwise keep waiting.
        *locked(&self.master) = None;
        {
            let mut core = locked(&self.core);
            core.subscribers.clear();
        }
        locked(&self.lifecycle).exit_code = Some(exit_code);
        self.lifecycle_changed.notify_all();
        self.sink
            .publish(self.agent_id, self.run, TerminalEvent::Exited { exit_code });
    }

    fn run_quiet_monitor(&self, quiet_period: Duration) {
        let tick = (quiet_period / 4).clamp(MIN_MONITOR_TICK, MAX_MONITOR_TICK);
        while !self.has_exited() {
            thread::sleep(tick);
            if self.take_quiet_period_report(quiet_period) {
                self.sink
                    .publish(self.agent_id, self.run, TerminalEvent::QuietPeriod);
            }
        }
    }

    /// True once per burst of output, when it has been silent for the quiet
    /// period.
    fn take_quiet_period_report(&self, quiet_period: Duration) -> bool {
        let mut core = locked(&self.core);
        let is_due = core.output_seen
            && !core.quiet_reported
            && core.last_output.elapsed() >= quiet_period
            && !self.has_exited();
        core.quiet_reported |= is_due;
        is_due
    }

    fn wait_for_exit(&self, timeout: Duration) -> bool {
        self.wait_until(timeout, |lifecycle| lifecycle.exit_code.is_some())
    }

    fn wait_for_reader(&self, timeout: Duration) {
        self.wait_until(timeout, |lifecycle| lifecycle.reader_finished);
    }

    fn wait_until(&self, timeout: Duration, is_done: impl Fn(&Lifecycle) -> bool) -> bool {
        let guard = locked(&self.lifecycle);
        let (guard, _) = self
            .lifecycle_changed
            .wait_timeout_while(guard, timeout, |lifecycle| !is_done(lifecycle))
            .unwrap_or_else(PoisonError::into_inner);
        is_done(&guard)
    }
}
