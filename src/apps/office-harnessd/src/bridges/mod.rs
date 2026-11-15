// SPDX-License-Identifier: AGPL-3.0-or-later

//! Adapters between modules. Modules never name each other's types, so the
//! host joins them here.

mod terminal_events;
mod terminal_launcher;

pub use terminal_events::{AgentTerminalEvent, TerminalEvents};
pub use terminal_launcher::{LaunchError, LaunchOutcome, LaunchRequest, TerminalLauncher};
