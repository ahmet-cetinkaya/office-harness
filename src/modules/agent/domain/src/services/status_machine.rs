// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::enums::{AgentStatus, StatusEvent};
use crate::errors::AgentError;

/// The status after `event`, or an error when the state machine of the data
/// model has no such transition.
///
/// # Errors
/// `AgentError::InvalidTransition` when `event` is not allowed in `status`.
pub fn next_status(status: AgentStatus, event: StatusEvent) -> Result<AgentStatus, AgentError> {
    use AgentStatus::{Exited, Failed, Interrupted, Starting, Stopped, WaitingForInput, Working};

    let next = match (status, event) {
        (Exited | Failed | Stopped | Interrupted, StatusEvent::Start) => Some(Starting),
        (Starting, StatusEvent::FirstOutput) | (WaitingForInput, StatusEvent::Output) => {
            Some(Working)
        }
        (Working, StatusEvent::QuietPeriodElapsed) => Some(WaitingForInput),
        (running, StatusEvent::ExitedWithCodeZero) if running.is_running() => Some(Exited),
        (running, StatusEvent::ExitedWithFailure) if running.is_running() => Some(Failed),
        (running, StatusEvent::StoppedByUser) if running.is_running() => Some(Stopped),
        (running, StatusEvent::ServerStopped) if running.is_running() => Some(Interrupted),
        _ => None,
    };
    next.ok_or(AgentError::InvalidTransition {
        from: status,
        event,
    })
}

#[cfg(test)]
#[path = "status_machine.tests.rs"]
mod tests;
