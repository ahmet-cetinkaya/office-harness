// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::interfaces::CliAdapter;
use agent_application::common::models::{SessionCommand, SessionPlan, SignIn};

use super::command_builder::{continue_command, new_session_command};

/// `gemini`: the id is chosen at the start with `--session-id`, and a
/// session continues with `--resume`.
/// Whether `--resume` takes the id, or only an index from `--list-sessions`,
/// is checked by hand in T7(g).
pub struct GeminiAdapter;

/// `gemini` has no sign-in status command; its sign-in screen asks how to
/// authenticate (0019 research R1, R4).
const SIGN_IN: SignIn = SignIn {
    cli: "gemini",
    status: None,
    screen_words: &[
        "how would you like to authenticate",
        "waiting for authentication",
    ],
};

impl CliAdapter for GeminiAdapter {
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
