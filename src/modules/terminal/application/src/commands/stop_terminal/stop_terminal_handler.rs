// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::time::Duration;

use kernel::AgentId;

use crate::common::errors::TerminalRuntimeError;
use crate::common::interfaces::TerminalRuntime;

pub struct StopTerminalHandler {
    runtime: Arc<dyn TerminalRuntime>,
}

impl StopTerminalHandler {
    #[must_use]
    pub fn new(runtime: Arc<dyn TerminalRuntime>) -> Self {
        Self { runtime }
    }

    /// # Errors
    /// The runtime's error.
    pub fn handle(&self, agent_id: AgentId, grace: Duration) -> Result<(), TerminalRuntimeError> {
        self.runtime.stop(agent_id, grace)
    }
}
