// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::interfaces::CliAdapter;
use agent_application::common::models::{SessionCommand, SessionPlan};

use super::command_builder::{continue_command, new_session_command};

/// `claude`: the id is chosen at the start with `--session-id`, and a
/// session continues with `--resume <id>` (research R3).
pub struct ClaudeAdapter;

impl CliAdapter for ClaudeAdapter {
    fn supports_continue(&self) -> bool {
        true
    }

    fn new_session(&self, plan: &SessionPlan<'_>) -> SessionCommand {
        new_session_command(
            plan,
            &["--session-id", plan.proposed_cli_session_id],
            Some(plan.proposed_cli_session_id.to_owned()),
        )
    }

    fn continue_session(
        &self,
        plan: &SessionPlan<'_>,
        cli_session_id: &str,
    ) -> Option<SessionCommand> {
        Some(continue_command(plan, &["--resume", cli_session_id]))
    }
}
