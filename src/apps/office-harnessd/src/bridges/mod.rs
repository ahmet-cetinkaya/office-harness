// SPDX-License-Identifier: AGPL-3.0-or-later

//! Adapters between modules. Modules never name each other's types, so the
//! host joins them here.

mod action_recorder;
mod agent_line_bridge;
mod agent_status_bridge;
mod directive_bridges;
mod hold_bridges;
mod needs_you_bridge;
mod notification_bridge;
mod quiet_period_source;
mod rate_limit_source;
mod role_requests;
mod saved_output;
mod subtree_requests;
mod system_clock;
mod terminal_events;
mod terminal_host;
mod terminal_launcher;
mod typed_input;

pub use action_recorder::{ActionRecorderBridge, take_recorded_events};
pub use agent_line_bridge::AgentLineBridge;
pub use agent_status_bridge::{AgentStatusBridge, StatusOrder, StoppingFlag};
pub use directive_bridges::{DirectiveJournalBridge, DirectiveTargetBridge, DirectiveWriterBridge};
pub use hold_bridges::{HoldJournalBridge, HoldNotifierBridge};
pub use needs_you_bridge::{AgentNames, NeedsYouBridge};
pub use notification_bridge::NotificationBridge;
pub use quiet_period_source::{QuietPeriodSource, SettingsQuietPeriod};
pub use rate_limit_source::SettingsRateLimits;
pub use role_requests::{AgentCreatorBridge, RoleRequestsBridge};
pub use saved_output::{SavedOutputError, SavedOutputs};
pub use subtree_requests::SubtreeRequestsBridge;
pub use system_clock::{SteadyClock, SystemClock};
pub use terminal_events::{AgentTerminalEvent, HeldLine, TerminalEvents};
pub use terminal_host::{AgentCalls, CallHelper, TerminalHostBridge};
pub use terminal_launcher::{
    LaunchError, LaunchOutcome, LaunchRequest, PrepareRequest, PreparedProgram, SpawnRequest,
    TerminalLauncher,
};
