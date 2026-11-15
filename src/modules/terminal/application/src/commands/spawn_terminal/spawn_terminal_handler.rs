// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use crate::commands::spawn_terminal::validate_spawn_terminal;
use crate::common::errors::TerminalRuntimeError;
use crate::common::interfaces::TerminalRuntime;
use crate::common::models::SpawnTerminalRequest;

pub struct SpawnTerminalHandler {
    runtime: Arc<dyn TerminalRuntime>,
}

impl SpawnTerminalHandler {
    #[must_use]
    pub fn new(runtime: Arc<dyn TerminalRuntime>) -> Self {
        Self { runtime }
    }

    /// # Errors
    /// A validation error that names the field, or the runtime's error.
    pub fn handle(&self, request: &SpawnTerminalRequest) -> Result<(), TerminalRuntimeError> {
        validate_spawn_terminal(request)?;
        self.runtime.spawn(request)
    }
}
