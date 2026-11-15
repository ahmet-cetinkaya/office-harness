// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::interfaces::CliAdapter;
use agent_application::common::models::{SessionCommand, SessionPlan};

use super::command_builder::{PromptStyle, continue_command, new_session_command};

/// `opencode`: `--session <id>` creates the session when it does not exist
/// and continues it when it does; the prompt is `--prompt`. The id format
/// that `opencode` accepts is checked by hand in T7(g).
pub struct OpencodeAdapter;

impl CliAdapter for OpencodeAdapter {
    fn supports_continue(&self) -> bool {
        true
    }

    fn new_session(&self, plan: &SessionPlan<'_>) -> SessionCommand {
        new_session_command(
            plan,
            &["--session", plan.proposed_cli_session_id],
            PromptStyle::Flag("--prompt"),
            Some(plan.proposed_cli_session_id.to_owned()),
        )
    }

    fn continue_session(
        &self,
        plan: &SessionPlan<'_>,
        cli_session_id: &str,
    ) -> Option<SessionCommand> {
        Some(continue_command(plan, &["--session", cli_session_id]))
    }
}
