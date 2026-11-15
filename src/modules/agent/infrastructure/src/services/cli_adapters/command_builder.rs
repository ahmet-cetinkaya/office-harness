// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::models::{SessionCommand, SessionPlan};

/// How a CLI takes the startup prompt on its command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptStyle {
    /// The last argument.
    Positional,
    /// The value of this flag.
    Flag(&'static str),
    /// The CLI has no argument for it, so it is typed in.
    Typed,
}

/// The command line of a new session: the fixed arguments, the flags, the
/// adapter's own arguments, and the prompt.
///
/// A prompt that starts with `-` would read as a flag, and a positional
/// prompt of one word (`logout`, `update`) as a subcommand, so both are typed
/// in instead, like a prompt for a batch script (D5).
#[must_use]
pub fn new_session_command(
    plan: &SessionPlan<'_>,
    own_arguments: &[&str],
    style: PromptStyle,
    cli_session_id: Option<String>,
) -> SessionCommand {
    let mut arguments = base_arguments(plan, own_arguments);
    let mut typed_input = None;
    if let Some(prompt) = plan.startup_prompt {
        let by_argument = plan.prompt_by_argument && !prompt.starts_with('-');
        match style {
            PromptStyle::Positional if by_argument && has_several_words(prompt) => {
                arguments.push(prompt.to_owned());
            }
            PromptStyle::Flag(flag) if by_argument => {
                arguments.push(flag.to_owned());
                arguments.push(prompt.to_owned());
            }
            _ => typed_input = Some(prompt.to_owned()),
        }
    }
    SessionCommand {
        arguments,
        cli_session_id,
        typed_input,
    }
}

fn has_several_words(prompt: &str) -> bool {
    prompt.split_whitespace().nth(1).is_some()
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
