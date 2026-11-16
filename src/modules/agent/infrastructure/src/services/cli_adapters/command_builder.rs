// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::models::{SessionCommand, SessionPlan};

/// The command line of a new session: the fixed arguments, the flags, and
/// the adapter's own arguments. The startup prompt is never an argument: it
/// is typed in after the first output, so it never reads as a flag or a
/// subcommand and never passes through a shell (D5).
#[must_use]
pub fn new_session_command(
    plan: &SessionPlan<'_>,
    own_arguments: &[&str],
    cli_session_id: Option<String>,
) -> SessionCommand {
    SessionCommand {
        arguments: base_arguments(plan, own_arguments),
        cli_session_id,
        typed_input: plan.startup_prompt.map(str::to_owned),
    }
}

/// The command line that continues a session. The startup prompt goes only
/// to a new session.
#[must_use]
pub fn continue_command(plan: &SessionPlan<'_>, own_arguments: &[&str]) -> SessionCommand {
    SessionCommand {
        arguments: base_arguments(plan, own_arguments),
        cli_session_id: None,
        typed_input: None,
    }
}

fn base_arguments(plan: &SessionPlan<'_>, own_arguments: &[&str]) -> Vec<String> {
    let mut arguments = plan.fixed_arguments.to_vec();
    arguments.extend(plan.flags.iter().cloned());
    arguments.extend(own_arguments.iter().map(|argument| (*argument).to_owned()));
    arguments
}

#[cfg(test)]
#[path = "command_builder.tests.rs"]
mod tests;
