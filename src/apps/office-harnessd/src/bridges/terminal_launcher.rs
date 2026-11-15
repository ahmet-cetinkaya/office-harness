// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;
use std::sync::Arc;

use agent_infrastructure::services::{Environment, LoginEnvironment, find_program};
use kernel::AgentId;
use terminal_application::common::errors::TerminalRuntimeError;
use terminal_application::common::interfaces::TerminalRuntime;
use terminal_application::common::models::SpawnTerminalRequest;
use terminal_domain::value_objects::{OutputKey, TerminalSize};
use thiserror::Error;

const TERMINAL_TYPE_VARIABLE: &str = "TERM";
const TERMINAL_TYPE: &str = "xterm-256color";
const COLOR_VARIABLE: &str = "COLORTERM";
const COLOR_SUPPORT: &str = "truecolor";

/// What starting one agent's terminal needs from the agent side.
#[derive(Debug, Clone)]
pub struct LaunchRequest {
    pub agent_id: AgentId,
    pub program: String,
    pub arguments: Vec<String>,
    pub working_folder: PathBuf,
    /// The office additions: applied last, so they win over the login
    /// environment (FR-020).
    pub additions: Vec<(String, String)>,
    pub size: TerminalSize,
    pub output_key: OutputKey,
}

/// A warning about the environment that the agent's details show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchOutcome {
    pub environment_warning: Option<String>,
}

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("the command {0} was not found on the login PATH")]
    ProgramNotFound(String),
    #[error(transparent)]
    Runtime(#[from] TerminalRuntimeError),
    #[error("the start task failed: {0}")]
    Task(String),
}

/// Starts a terminal the way FR-020 asks: resolve the login environment,
/// apply the office additions, find the command on the resulting `PATH`, and
/// then spawn. The resolution runs a shell and the spawn blocks, so both run
/// on a blocking thread.
#[derive(Clone)]
pub struct TerminalLauncher {
    runtime: Arc<dyn TerminalRuntime>,
    login_environment: Arc<LoginEnvironment>,
    server_environment: Arc<Environment>,
}

impl TerminalLauncher {
    #[must_use]
    pub fn new(
        runtime: Arc<dyn TerminalRuntime>,
        login_environment: Arc<LoginEnvironment>,
        server_environment: Environment,
    ) -> Self {
        Self {
            runtime,
            login_environment,
            server_environment: Arc::new(server_environment),
        }
    }

    /// # Errors
    /// `ProgramNotFound`, or the runtime's error (for example a command that
    /// cannot start, or an agent that already runs).
    pub async fn launch(&self, request: LaunchRequest) -> Result<LaunchOutcome, LaunchError> {
        let launcher = self.clone();
        tokio::task::spawn_blocking(move || launcher.launch_blocking(request))
            .await
            .map_err(|error| LaunchError::Task(error.to_string()))?
    }

    /// The same as `launch`, for a caller that already runs on a blocking
    /// thread.
    ///
    /// # Errors
    /// As `launch`.
    pub fn launch_blocking(&self, request: LaunchRequest) -> Result<LaunchOutcome, LaunchError> {
        let resolved = self.login_environment.resolve(&self.server_environment);
        let mut environment = resolved.variables;
        // The PTY host names the terminal, as every terminal emulator does; a
        // service has no terminal of its own to inherit this from. The office
        // additions come after and may replace both.
        environment.insert(TERMINAL_TYPE_VARIABLE.to_owned(), TERMINAL_TYPE.to_owned());
        environment.insert(COLOR_VARIABLE.to_owned(), COLOR_SUPPORT.to_owned());
        environment.extend(request.additions);
        let program = find_program(&request.program, &environment, &request.working_folder)
            .ok_or_else(|| LaunchError::ProgramNotFound(request.program.clone()))?;
        self.runtime.spawn(&SpawnTerminalRequest {
            agent_id: request.agent_id,
            program: program.to_string_lossy().into_owned(),
            arguments: request.arguments,
            working_folder: request.working_folder,
            environment: environment.into_iter().collect(),
            size: request.size,
            output_key: request.output_key,
        })?;
        Ok(LaunchOutcome {
            environment_warning: resolved.warning,
        })
    }
}
