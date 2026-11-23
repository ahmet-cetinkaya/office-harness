// SPDX-License-Identifier: AGPL-3.0-or-later

//! Adapters between modules. Modules never name each other's types, so the
//! host joins them here.

mod action_recorder;
mod agent_definition_bridge;
mod agent_line_bridge;
mod agent_status_bridge;
mod directive_bridges;
mod hold_bridges;
mod hook_sessions;
mod hook_status_bridge;
mod idle_sleep_gate;
mod needs_you_bridge;
mod notification_bridge;
mod person_input;
mod person_reports;
mod process_probe_bridge;
mod rate_limit_source;
mod role_requests;
mod saved_output;
mod sign_in_watch;
mod status_command_bridge;
mod subtree_requests;
mod system_clock;
mod temp_ender;
mod temp_endings;
mod temp_hooks;
mod temp_ledger;
mod template_create;
mod template_create_error;
mod template_plan;
mod terminal_events;
mod terminal_host;
mod terminal_launcher;
mod typed_input;
mod wait_tracker;

pub use action_recorder::{ActionRecorderBridge, take_recorded_events};
pub use agent_definition_bridge::AgentDefinitionBridge;
pub use agent_line_bridge::AgentLineBridge;
pub use agent_status_bridge::{AgentStatusBridge, StatusOrder, StoppingFlag};
pub use directive_bridges::{
    DirectiveJournalBridge, DirectiveReceiptBridge, DirectiveTargetBridge, DirectiveWriterBridge,
    TaskOrder,
};
pub use hold_bridges::{HoldJournalBridge, HoldNotifierBridge};
pub use hook_sessions::{
    DETAIL_PERMISSION_ASKED, DETAIL_TURN_FAILED, DETAIL_TURN_STOPPED, HookSessions, HookState,
    StatusSource,
};
pub use hook_status_bridge::HookStatusBridge;
pub use idle_sleep_gate::{GateInputs, IdleRule, IdleSleepGate, SleepingAgents};
pub use needs_you_bridge::{AgentNames, NeedsYouBridge};
pub use notification_bridge::NotificationBridge;
pub use person_input::{PERSON_TYPING_PAUSE, PersonInput};
pub use person_reports::PersonReportsBridge;
pub use process_probe_bridge::ProcessProbeBridge;
pub use rate_limit_source::SettingsRateLimits;
pub use role_requests::{AgentCreatorBridge, RoleRequestsBridge};
pub use saved_output::{SavedOutputError, SavedOutputs};
pub use sign_in_watch::{SignInEnds, SignInWatch, SignInWatchTiming};
pub use status_command_bridge::StatusCommandBridge;
pub use subtree_requests::{
    SubtreeRequestsBridge, delete_worktrees, subtree_worktrees, worktree_report,
};
pub use system_clock::{SteadyClock, SystemClock};
pub use temp_ender::{EndOrder, EndOutcome, EndReason, TempEnder};
pub use temp_endings::{CreatingGuard, END_CHECK_REQUEST, ENDING_MESSAGE, TempEndings};
pub use temp_hooks::TempHooks;
pub use temp_ledger::{
    EndNote, KeptWork, TEMP_CHANGED_EVENT, TempLedger, TempSnapshot, temp_changed_event,
};
pub use template_create::{CreatedAgent, DirectPlace, TeamCreation, TemplateCreateBridge};
pub use template_create_error::TemplateCreateError;
pub use terminal_events::{AgentTerminalEvent, HeldLine, TerminalEvents};
pub use terminal_host::{AgentCalls, CallHelper, TerminalHostBridge};
pub use terminal_launcher::{
    LaunchError, LaunchOutcome, LaunchRequest, PrepareRequest, PreparedProgram, SpawnRequest,
    TerminalLauncher,
};
pub use wait_tracker::{Change, StatusChange, WORK_BOUND_EXTRA, WaitTracker};
