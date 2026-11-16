// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_domain::entities::Session;
use agent_domain::value_objects::{AgentCommand, ExitResult};

use crate::common::command_line::split_command;
use crate::common::errors::ApplicationError;
use crate::common::interfaces::{CliAdapter, CliAdapters};
use crate::common::models::SessionView;

/// The adapter of the CLI that an agent's command starts.
///
/// # Errors
/// `Invalid` when the command cannot be split.
pub fn adapter_of(
    adapters: &dyn CliAdapters,
    command: &AgentCommand,
) -> Result<Arc<dyn CliAdapter>, ApplicationError> {
    let (program, _) = split_command(command)?;
    Ok(adapters.for_program(&program))
}

/// The CLI's own session id when the user may continue `session`, or the
/// reason why not (FR-025). The reason is never empty.
///
/// # Errors
/// The reason, as text to show.
pub fn continuation_of<'a>(
    session: &'a Session,
    adapter: &dyn CliAdapter,
) -> Result<&'a str, String> {
    if !adapter.supports_continue() {
        return Err("the agent's command does not support continuing a session".to_owned());
    }
    if session.is_running() {
        return Err("the session is still running".to_owned());
    }
    if matches!(session.exit_result, Some(ExitResult::SpawnError(_))) {
        return Err("the session never started".to_owned());
    }
    session.cli_session_id.as_deref().ok_or_else(|| {
        "office-harness could not tell which of the CLI's sessions belongs to this one, \
         so it does not guess"
            .to_owned()
    })
}

/// The view of a session that a start just made: it runs, or it never
/// started, so it cannot be continued yet.
#[must_use]
pub fn fresh_session_view(session: Session) -> SessionView {
    let reason = if session.is_running() {
        "the session is still running"
    } else {
        "the session never started"
    };
    SessionView {
        cannot_continue_reason: Some(reason.to_owned()),
        session,
    }
}
