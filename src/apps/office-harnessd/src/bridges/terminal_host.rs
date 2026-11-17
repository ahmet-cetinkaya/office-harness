// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::time::Duration;

use access_application::common::interfaces::AgentCallStore;
use agent_application::common::errors::TerminalHostError;
use agent_application::common::interfaces::TerminalHost;
use agent_application::common::models::{PrepareLaunch, PreparedLaunch, StartTerminal};
use kernel::AgentId;
use terminal_application::common::errors::TerminalRuntimeError;
use terminal_application::common::interfaces::TerminalRuntime;
use terminal_domain::errors::TerminalError;
use terminal_domain::services::{
    first_unsafe_batch_argument, first_unsafe_batch_character, is_batch_script,
};
use terminal_domain::value_objects::{OutputKey, TerminalSize};

use super::terminal_events::TerminalEvents;
use super::terminal_launcher::{LaunchError, PrepareRequest, SpawnRequest, TerminalLauncher};
use super::typed_input::type_after_first_output;
use crate::bootstrap::ServerLog;

/// The size of a terminal until the first client attaches and resizes it.
const DEFAULT_COLUMNS: u16 = 120;
const DEFAULT_ROWS: u16 = 30;

/// The agent module's `TerminalHost`, backed by the terminal module. It
/// names no agent type to the terminal and no terminal type to the agent.
pub struct TerminalHostBridge {
    launcher: TerminalLauncher,
    runtime: Arc<dyn TerminalRuntime>,
    events: TerminalEvents,
    log: ServerLog,
    /// Where each agent's call credential is issued (0002).
    calls: Arc<dyn AgentCallStore>,
    /// The two values that tell an agent how to call the office back: the
    /// server binary and the file that names its port.
    call_helper: CallHelper,
}

/// What an agent session needs to call the office back: the store that
/// issues its credential and the helper that performs the call.
pub struct AgentCalls {
    pub store: Arc<dyn AgentCallStore>,
    pub helper: CallHelper,
}

/// How an agent finds the office again from inside its session.
#[derive(Debug, Clone)]
pub struct CallHelper {
    pub server_binary: String,
    pub server_file: String,
}

/// The environment names of the call credential (contract, Environment).
pub const AGENT_TOKEN_VARIABLE: &str = "OFFICE_HARNESS_AGENT_TOKEN";
pub const CLI_VARIABLE: &str = "OFFICE_HARNESS_CLI";
pub const SERVER_FILE_VARIABLE: &str = "OFFICE_HARNESS_SERVER_FILE";

impl TerminalHostBridge {
    #[must_use]
    pub fn new(
        launcher: TerminalLauncher,
        runtime: Arc<dyn TerminalRuntime>,
        events: TerminalEvents,
        log: ServerLog,
        agent_calls: AgentCalls,
    ) -> Self {
        Self {
            launcher,
            runtime,
            events,
            log,
            calls: agent_calls.store,
            call_helper: agent_calls.helper,
        }
    }
}

fn failed(error: &dyn std::fmt::Display) -> TerminalHostError {
    TerminalHostError::Failed(error.to_string())
}

impl TerminalHost for TerminalHostBridge {
    fn prepare(&self, request: &PrepareLaunch) -> Result<PreparedLaunch, TerminalHostError> {
        // A new session gets a new credential, so the one of an earlier
        // session never works again. The credential is in this session's
        // environment and nowhere else.
        let credential = self
            .calls
            .issue(request.agent_id, &request.session_id.to_string())
            .map_err(|error| failed(&error))?;
        let mut additions = request.environment_additions.clone();
        additions.push((
            AGENT_TOKEN_VARIABLE.to_owned(),
            format!("{}:{}", request.agent_id, credential.token.expose_secret()),
        ));
        additions.push((
            CLI_VARIABLE.to_owned(),
            self.call_helper.server_binary.clone(),
        ));
        additions.push((
            SERVER_FILE_VARIABLE.to_owned(),
            self.call_helper.server_file.clone(),
        ));
        let prepared = self
            .launcher
            .prepare(&PrepareRequest {
                program: request.program.clone(),
                working_folder: request.working_folder.clone(),
                additions,
            })
            .inspect_err(|_| {
                self.calls
                    .revoke_session(request.agent_id, &request.session_id.to_string());
            })
            .map_err(|error| failed(&error))?;
        let program = prepared.program.to_string_lossy().into_owned();
        Ok(PreparedLaunch {
            is_batch_script: is_batch_script(&program),
            program,
            environment: prepared.environment,
        })
    }

    fn validate_arguments(
        &self,
        prepared: &PreparedLaunch,
        arguments: &[String],
    ) -> Result<(), TerminalHostError> {
        if !prepared.is_batch_script {
            return Ok(());
        }
        if let Some(character) = first_unsafe_batch_character(&prepared.program) {
            return Err(refused(&TerminalError::UnsafeBatchProgram { character }));
        }
        match first_unsafe_batch_argument(arguments) {
            Some((index, character)) => Err(refused(&TerminalError::UnsafeBatchArgument {
                index,
                character,
            })),
            None => Ok(()),
        }
    }

    fn start(&self, request: &StartTerminal) -> Result<(), TerminalHostError> {
        let size = TerminalSize::new(DEFAULT_COLUMNS, DEFAULT_ROWS).map_err(|e| failed(&e))?;
        // The session's output file is named by its id.
        let output_key =
            OutputKey::parse(&request.session_id.to_string()).map_err(|e| failed(&e))?;
        // Subscribed before the start, so the first output is not missed.
        let first_output = request
            .typed_input
            .as_ref()
            .map(|_| self.events.subscribe());
        self.launcher
            .spawn_prepared(SpawnRequest {
                agent_id: request.agent_id,
                program: request.program.clone(),
                arguments: request.arguments.clone(),
                working_folder: request.working_folder.clone(),
                environment: request.environment.clone(),
                size,
                output_key,
            })
            .inspect_err(|_| {
                self.calls
                    .revoke_session(request.agent_id, &request.session_id.to_string());
            })
            .map_err(|error| match error {
                LaunchError::Runtime(TerminalRuntimeError::AlreadyRunning(_)) => {
                    TerminalHostError::AlreadyRunning
                }
                other => failed(&other),
            })?;
        if let (Some(text), Some(events)) = (request.typed_input.clone(), first_output) {
            type_after_first_output(
                Arc::clone(&self.runtime),
                events,
                request.agent_id,
                text,
                self.log.clone(),
            );
        }
        Ok(())
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

fn refused(error: &TerminalError) -> TerminalHostError {
    TerminalHostError::Refused(error.to_string())
}
