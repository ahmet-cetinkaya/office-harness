// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::interfaces::CliAdapter;
use agent_application::common::models::{SessionCommand, SessionPlan};

use super::command_builder::{PromptStyle, continue_command, new_session_command};

/// `gemini`: the id is chosen at the start with `--session-id`, the prompt
/// stays interactive with `-i`, and a session continues with `--resume`.
/// Whether `--resume` takes the id, or only an index from `--list-sessions`,
/// is checked by hand in T7(g).
pub struct GeminiAdapter;

impl CliAdapter for GeminiAdapter {
    fn supports_continue(&self) -> bool {
        true
    }

    fn new_session(&self, plan: &SessionPlan<'_>) -> SessionCommand {
        new_session_command(
            plan,
            &["--session-id", plan.proposed_cli_session_id],
            PromptStyle::Flag("-i"),
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
