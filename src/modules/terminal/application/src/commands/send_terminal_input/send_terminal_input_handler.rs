// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::AgentId;

use crate::common::errors::TerminalRuntimeError;
use crate::common::interfaces::TerminalRuntime;

pub struct SendTerminalInputHandler {
    runtime: Arc<dyn TerminalRuntime>,
}

impl SendTerminalInputHandler {
    #[must_use]
    pub fn new(runtime: Arc<dyn TerminalRuntime>) -> Self {
        Self { runtime }
    }

    /// Keys reach the child unchanged.
    ///
    /// # Errors
    /// The runtime's error.
    pub fn handle(&self, agent_id: AgentId, bytes: &[u8]) -> Result<(), TerminalRuntimeError> {
        self.runtime.write_input(agent_id, bytes)
    }
}
