// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_application::common::interfaces::ProcessProbe;
use agent_application::common::models::{ProbeError, SessionProcess};
use kernel::AgentId;
use terminal_application::common::interfaces::{ProcessInspector, ReadError, TerminalRuntime};
use terminal_application::common::models::ProcessIdentity;

/// The agent module's `ProcessProbe`, backed by the terminal module: the
/// runtime names the CLI process of an agent, and the inspector reads
/// processes. The probe only reads; it ends nothing.
pub struct ProcessProbeBridge {
    runtime: Arc<dyn TerminalRuntime>,
    inspector: Arc<dyn ProcessInspector>,
}

impl ProcessProbeBridge {
    #[must_use]
    pub fn new(runtime: Arc<dyn TerminalRuntime>, inspector: Arc<dyn ProcessInspector>) -> Self {
        Self { runtime, inspector }
    }
}

impl ProcessProbe for ProcessProbeBridge {
    fn cli_process_id(&self, agent_id: AgentId) -> Option<u32> {
        self.runtime.process_id(agent_id)
    }

    fn read(&self, pid: u32) -> Result<SessionProcess, ProbeError> {
        let handle = self.inspector.open(pid).map_err(probe_error)?;
        handle.identity().map(session_process).map_err(probe_error)
    }

    fn descendants(&self, root: u32) -> Vec<u32> {
        self.inspector.descendants(root)
    }
}

fn probe_error(error: ReadError) -> ProbeError {
    match error {
        ReadError::NotRunning => ProbeError::NotRunning,
        ReadError::NotReadable => ProbeError::NotReadable,
    }
}

fn session_process(identity: ProcessIdentity) -> SessionProcess {
    SessionProcess {
        pid: identity.pid,
        started_at: identity.started_at,
        command_hash: identity.command_hash,
        command_display: identity.command_display,
        boot: identity.boot,
    }
}

#[cfg(test)]
#[path = "process_probe_bridge.tests.rs"]
mod tests;
