// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::interfaces::CliAdapter;
use agent_application::common::models::{SessionCommand, SessionPlan};

use super::command_builder::new_session_command;

/// Any other command: it runs as typed, gets the startup prompt as typed
/// input after its first output, and cannot continue a session (FR-025).
pub struct GenericAdapter;

impl CliAdapter for GenericAdapter {
    fn supports_continue(&self) -> bool {
        false
    }

    fn new_session(&self, plan: &SessionPlan<'_>) -> SessionCommand {
        new_session_command(plan, &[], None)
    }

    fn continue_session(
        &self,
        _plan: &SessionPlan<'_>,
        _cli_session_id: &str,
    ) -> Option<SessionCommand> {
        None
    }
}
