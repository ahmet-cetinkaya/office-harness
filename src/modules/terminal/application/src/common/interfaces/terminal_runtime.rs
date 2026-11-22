// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::time::Duration;

use kernel::AgentId;
use terminal_domain::value_objects::{OutputKey, TerminalSize};

use crate::common::errors::TerminalRuntimeError;
use crate::common::interfaces::OutputSubscriber;
use crate::common::models::{Attachment, RunId, SpawnTerminalRequest, SubscriptionId};

/// The process side of the terminal module: one PTY and child per agent.
/// Operations block, so an async caller runs them on a blocking thread.
pub trait TerminalRuntime: Send + Sync {
    /// Starts the command in a new PTY.
    ///
    /// # Errors
    /// `AlreadyRunning` when the agent has a running terminal, `SpawnFailed`
    /// when the command cannot start.
    fn spawn(&self, request: &SpawnTerminalRequest) -> Result<(), TerminalRuntimeError>;

    /// Resizes to `size` (a no-op when equal), takes the snapshot, and, for a
    /// running terminal, subscribes `subscriber`, all in one step. For an
    /// agent that is not running, the snapshot is the saved output named by
    /// `saved_output`, and no subscription is made.
    ///
    /// # Errors
    /// `NoOutput` when the agent is not running and nothing is saved.
    fn attach(
        &self,
        agent_id: AgentId,
        saved_output: Option<&OutputKey>,
        size: TerminalSize,
        subscriber: Arc<dyn OutputSubscriber>,
    ) -> Result<Attachment, TerminalRuntimeError>;

    fn detach(&self, agent_id: AgentId, subscription: SubscriptionId);

    /// Writes the bytes to the child unchanged.
    ///
    /// # Errors
    /// `NotRunning` or `Io`.
    fn write_input(&self, agent_id: AgentId, bytes: &[u8]) -> Result<(), TerminalRuntimeError>;

    /// Writes text as a paste: wrapped in the bracketed-paste markers when the
    /// child has turned that mode on, so a line break does not submit early.
    ///
    /// # Errors
    /// `NotRunning` or `Io`.
    fn paste(&self, agent_id: AgentId, text: &str) -> Result<(), TerminalRuntimeError>;

    /// Returns whether the size changed. A resize to the current size is
    /// ignored and sends nothing to the child.
    ///
    /// # Errors
    /// `NotRunning` or `Io`.
    fn resize(&self, agent_id: AgentId, size: TerminalSize) -> Result<bool, TerminalRuntimeError>;

    /// Ends the child and everything it started: it asks, waits `grace`, and
    /// then forces.
    ///
    /// # Errors
    /// `NotRunning`, or `StopFailed` when the child is still alive.
    fn stop(&self, agent_id: AgentId, grace: Duration) -> Result<(), TerminalRuntimeError>;

    fn is_running(&self, agent_id: AgentId) -> bool;

    /// The run of the agent's latest start, also after its child exited, or
    /// `None` when it never started.
    fn current_run(&self, agent_id: AgentId) -> Option<RunId>;

    /// How long the running child has written nothing: since its last output,
    /// or since its start when it wrote nothing yet. `None` when it does not
    /// run. Only the time leaves the terminal, never the output (0014).
    fn quiet_for(&self, agent_id: AgentId) -> Option<Duration>;
}
