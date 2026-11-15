// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::AgentId;
use terminal_domain::value_objects::TerminalSize;

use crate::common::errors::TerminalRuntimeError;
use crate::common::interfaces::TerminalRuntime;

pub struct ResizeTerminalHandler {
    runtime: Arc<dyn TerminalRuntime>,
}

impl ResizeTerminalHandler {
    #[must_use]
    pub fn new(runtime: Arc<dyn TerminalRuntime>) -> Self {
        Self { runtime }
    }

    /// Returns whether the size changed; the last resize wins.
    ///
    /// # Errors
    /// The runtime's error.
    pub fn handle(
        &self,
        agent_id: AgentId,
        size: TerminalSize,
    ) -> Result<bool, TerminalRuntimeError> {
        self.runtime.resize(agent_id, size)
    }
}
