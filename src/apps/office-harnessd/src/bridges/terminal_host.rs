// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::time::Duration;

use agent_application::common::errors::TerminalHostError;
use agent_application::common::interfaces::TerminalHost;
use agent_application::common::models::StartTerminal;
use kernel::AgentId;
use terminal_application::common::errors::TerminalRuntimeError;
use terminal_application::common::interfaces::TerminalRuntime;
use terminal_domain::value_objects::{OutputKey, TerminalSize};

use super::terminal_launcher::{LaunchError, LaunchRequest, TerminalLauncher};

/// The size of a terminal until the first client attaches and resizes it.
const DEFAULT_COLUMNS: u16 = 120;
const DEFAULT_ROWS: u16 = 30;

/// The agent module's `TerminalHost`, backed by the terminal module. It
/// names no agent type to the terminal and no terminal type to the agent.
pub struct TerminalHostBridge {
    launcher: TerminalLauncher,
    runtime: Arc<dyn TerminalRuntime>,
}

impl TerminalHostBridge {
    #[must_use]
    pub fn new(launcher: TerminalLauncher, runtime: Arc<dyn TerminalRuntime>) -> Self {
        Self { launcher, runtime }
    }
}

impl TerminalHost for TerminalHostBridge {
    fn start(&self, request: &StartTerminal) -> Result<(), TerminalHostError> {
        let failed = |error: &dyn std::fmt::Display| TerminalHostError::Failed(error.to_string());
        let size = TerminalSize::new(DEFAULT_COLUMNS, DEFAULT_ROWS).map_err(|e| failed(&e))?;
        // One output file for each agent until sessions exist (change 0001, T7).
        let output_key = OutputKey::parse(&request.agent_id.to_string()).map_err(|e| failed(&e))?;
        self.launcher
            .launch_blocking(LaunchRequest {
                agent_id: request.agent_id,
                program: request.program.clone(),
                arguments: request.arguments.clone(),
                working_folder: request.working_folder.clone(),
                additions: Vec::new(),
                size,
                output_key,
            })
            .map(|_| ())
            .map_err(|error| match error {
                LaunchError::Runtime(TerminalRuntimeError::AlreadyRunning(_)) => {
                    TerminalHostError::AlreadyRunning
                }
                other => failed(&other),
            })
    }

    fn stop(&self, agent_id: AgentId, grace: Duration) -> Result<(), TerminalHostError> {
        self.runtime
            .stop(agent_id, grace)
            .map_err(|error| match error {
                TerminalRuntimeError::NotRunning(_) => TerminalHostError::NotRunning,
                other => TerminalHostError::Failed(other.to_string()),
            })
    }
}
