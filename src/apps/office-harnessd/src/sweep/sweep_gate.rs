// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use serde_json::{Map, json};
use tokio::sync::watch;
use wire::Event;

const SWEEP_EVENT: &str = "office.sweep";

/// What clients are told about the sweep (`office.sweep`, `sweep.state`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SweepState {
    Running,
    Done,
}

impl SweepState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Done => "done",
        }
    }
}

/// The `office.sweep` event for a change of the sweep state.
#[must_use]
pub fn sweep_event(state: SweepState) -> Event {
    let mut fields = Map::new();
    fields.insert("state".to_owned(), json!(state.as_str()));
    Event::new(SWEEP_EVENT, fields)
}

/// The gate state. `Stopped` is the server stopping while the sweep still
/// runs: the requests that wait must stop waiting and be refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GateState {
    Running,
    Done,
    Stopped,
}

/// The server stopped while a request waited for the sweep.
#[derive(Debug, PartialEq, Eq)]
pub struct ServerStopping;

/// Holds the user's starting requests and the resume plan until the sweep is
/// over (0017/FR-007). It starts `running` only when the sweep has something
/// to do, and it never goes back to `running`.
#[derive(Clone)]
pub struct SweepGate {
    state: Arc<watch::Sender<GateState>>,
}

impl SweepGate {
    /// A gate for a sweep that runs now (`true`), or for no sweep (`false`).
    #[must_use]
    pub fn new(is_sweep_pending: bool) -> Self {
        let state = if is_sweep_pending {
            GateState::Running
        } else {
            GateState::Done
        };
        Self {
            state: Arc::new(watch::Sender::new(state)),
        }
    }

    #[must_use]
    pub fn state(&self) -> SweepState {
        match *self.state.borrow() {
            GateState::Running => SweepState::Running,
            GateState::Done | GateState::Stopped => SweepState::Done,
        }
    }

    /// The sweep is over; the waiting requests run.
    pub fn finish(&self) {
        self.leave_running(GateState::Done);
    }

    /// The server stops; the waiting requests are refused.
    pub fn stop(&self) {
        self.leave_running(GateState::Stopped);
    }

    /// Completes when the sweep is over.
    ///
    /// # Errors
    /// `ServerStopping` when the server stops first.
    pub async fn wait_until_done(&self) -> Result<(), ServerStopping> {
        let mut states = self.state.subscribe();
        match states.wait_for(|state| *state != GateState::Running).await {
            Ok(state) if *state == GateState::Done => Ok(()),
            _ => Err(ServerStopping),
        }
    }

    fn leave_running(&self, next: GateState) {
        self.state.send_if_modified(|state| {
            let was_running = *state == GateState::Running;
            if was_running {
                *state = next;
            }
            was_running
        });
    }
}

impl Default for SweepGate {
    fn default() -> Self {
        Self::new(false)
    }
}

#[cfg(test)]
#[path = "sweep_gate.tests.rs"]
mod tests;
