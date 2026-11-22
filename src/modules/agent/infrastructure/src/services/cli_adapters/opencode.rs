// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::interfaces::CliAdapter;
use agent_application::common::models::{SessionCommand, SessionPlan};

use super::command_builder::{continue_command, new_session_command};

/// `opencode` refuses a session id that does not start with this prefix
/// ("Expected a string starting with \"ses\"", version 2.2.5).
const SESSION_ID_PREFIX: &str = "ses_";

/// `opencode`: `--session <id>` creates the session when it does not exist
/// and continues it when it does. The id is the proposed id in the CLI's
/// own shape: the prefix, then the proposed id without its hyphens, because
/// the CLI's own ids hold only letters and digits after the prefix.
pub struct OpencodeAdapter;

impl CliAdapter for OpencodeAdapter {
    fn supports_continue(&self) -> bool {
        true
    }

    fn new_session(&self, plan: &SessionPlan<'_>) -> SessionCommand {
        let cli_session_id = opencode_session_id(plan.proposed_cli_session_id);
        new_session_command(
            plan,
            &["--session", &cli_session_id],
            Some(cli_session_id.clone()),
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

fn opencode_session_id(proposed_cli_session_id: &str) -> String {
    format!(
        "{SESSION_ID_PREFIX}{}",
        proposed_cli_session_id.replace('-', "")
    )
}
