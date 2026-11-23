// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::interfaces::CliAdapter;
use agent_application::common::models::{SessionCommand, SessionPlan, SignIn, StatusCommand};

use super::command_builder::{continue_command, new_session_command};

/// `claude`: the id is chosen at the start with `--session-id`, and a
/// session continues with `--resume <id>` (research R3).
pub struct ClaudeAdapter;

/// `claude auth status --text` ends with exit 1 and `Not logged in` when the
/// CLI is not signed in; its sign-in screen offers the two kinds of account
/// or asks to run `/login` (0019 research R1, R4).
const SIGN_IN: SignIn = SignIn {
    cli: "claude",
    status: Some(StatusCommand {
        arguments: &["auth", "status", "--text"],
        not_signed_in_words: &["not logged in"],
        key_variables: &[
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_AUTH_TOKEN",
            "CLAUDE_CODE_OAUTH_TOKEN",
            "CLAUDE_CODE_USE_BEDROCK",
            "CLAUDE_CODE_USE_VERTEX",
        ],
    }),
    screen_words: &[
        "claude account with subscription",
        "anthropic console account",
        "run /login",
    ],
};

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

    fn sign_in(&self) -> SignIn {
        SIGN_IN
    }
}
