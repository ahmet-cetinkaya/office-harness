// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use kernel::AgentId;

use crate::common::errors::TerminalHostError;
use crate::common::models::StartTerminal;

/// The terminal that runs an agent's CLI. The host app implements it with the
/// terminal module (the bridge). Operations block, so an async caller runs
/// them on a blocking thread.
pub trait TerminalHost: Send + Sync {
    /// Starts the command. The terminal's events later drive the status.
    ///
    /// # Errors
    /// `AlreadyRunning`, or `Failed` with the text to show (for example a
    /// command that is not on the login `PATH`).
    fn start(&self, request: &StartTerminal) -> Result<(), TerminalHostError>;

    /// Ends the CLI and everything it started: asks, waits `grace`, then
    /// forces.
    ///
    /// # Errors
    /// `NotRunning`, or `Failed` when the child is still alive.
    fn stop(&self, agent_id: AgentId, grace: Duration) -> Result<(), TerminalHostError>;
}
