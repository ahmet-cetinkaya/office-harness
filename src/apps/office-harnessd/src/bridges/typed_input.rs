// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::thread;

use kernel::AgentId;
use terminal_application::common::errors::TerminalRuntimeError;
use terminal_application::common::interfaces::TerminalRuntime;
use terminal_application::common::models::RunId;
use terminal_domain::events::TerminalEvent;
use tokio::sync::broadcast::{Receiver, error::RecvError};

use super::person_input::PersonInput;
use super::terminal_events::AgentTerminalEvent;
use crate::bootstrap::ServerLog;

/// Types `text` into the agent's terminal after the run's first output, as a
/// paste: wrapped in the bracketed-paste markers when the CLI turned that
/// mode on, so a line break does not submit early. The text is never
/// submitted: the first screen of a CLI may be a trust or permission dialog,
/// and an Enter from here could accept it, so the user submits. The text
/// never passes through a shell or a command line (D5). `events` must have
/// been subscribed before the run started, so its first output is not
/// missed. A failure other than a child that ended is logged. A pasted
/// prompt waits for the person, so `person_input` holds directives back
/// until the person types (0012 A11).
pub fn type_after_first_output(
    runtime: Arc<dyn TerminalRuntime>,
    mut events: Receiver<AgentTerminalEvent>,
    agent_id: AgentId,
    text: String,
    log: ServerLog,
    person_input: PersonInput,
) {
    let Some(run) = runtime.current_run(agent_id) else {
        return;
    };
    thread::spawn(move || {
        if !wait_for_first_output(&mut events, agent_id, run) {
            return;
        }
        match paste_into_run(&*runtime, agent_id, run, &text) {
            Ok(true) => person_input.office_text_left(agent_id, run),
            Ok(false) => {}
            Err(error) => log.error(&format!(
                "the startup prompt could not be typed in: {error}"
            )),
        }
    });
}

/// Pastes only into the run that asked for it: a newer start of the agent
/// must not receive the earlier run's prompt. A child that ended meanwhile
/// has nobody to type to, which is not a failure. True when it pasted.
fn paste_into_run(
    runtime: &dyn TerminalRuntime,
    agent_id: AgentId,
    run: RunId,
    text: &str,
) -> Result<bool, TerminalRuntimeError> {
    if runtime.current_run(agent_id) != Some(run) {
        return Ok(false);
    }
    match runtime.paste(agent_id, text) {
        Ok(()) => Ok(true),
        Err(TerminalRuntimeError::NotRunning(_)) => Ok(false),
        Err(error) => Err(error),
    }
}

/// True when the run produced output, and false when it ended first or the
/// events stopped.
fn wait_for_first_output(
    events: &mut Receiver<AgentTerminalEvent>,
    agent_id: AgentId,
    run: RunId,
) -> bool {
    loop {
        match events.blocking_recv() {
            Ok(item) if item.agent_id == agent_id && item.run == run => match item.event {
                TerminalEvent::FirstOutput => return true,
                TerminalEvent::Exited { .. } => return false,
                _ => {}
            },
            Ok(_) => {}
            // Events were dropped: the output has begun by now.
            Err(RecvError::Lagged(_)) => return true,
            Err(RecvError::Closed) => return false,
        }
    }
}

#[cfg(test)]
#[path = "typed_input.tests.rs"]
mod tests;
