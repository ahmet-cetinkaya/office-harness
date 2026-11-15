// SPDX-License-Identifier: AGPL-3.0-or-later

//! Adapters between modules. Modules never name each other's types, so the
//! host joins them here.

mod action_recorder;
mod agent_status_bridge;
mod saved_output;
mod system_clock;
mod terminal_events;
mod terminal_host;
mod terminal_launcher;
mod typed_input;

pub use action_recorder::{ActionRecorderBridge, take_recorded_events};
pub use agent_status_bridge::{AgentStatusBridge, StatusOrder, StoppingFlag};
pub use saved_output::{SavedOutputError, SavedOutputs};
pub use system_clock::SystemClock;
pub use terminal_events::{AgentTerminalEvent, TerminalEvents};
pub use terminal_host::TerminalHostBridge;
pub use terminal_launcher::{
    LaunchError, LaunchOutcome, LaunchRequest, PrepareRequest, PreparedProgram, SpawnRequest,
    TerminalLauncher,
};
