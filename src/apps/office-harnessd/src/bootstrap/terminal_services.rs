// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::time::Duration;

use agent_domain::value_objects::Settings;
use agent_infrastructure::services::{Environment, LoginEnvironment};
use terminal_application::common::interfaces::{TerminalEventSink, TerminalRuntime};
use terminal_domain::errors::TerminalError;
use terminal_domain::value_objects::OutputLimits;
use terminal_infrastructure::services::{PortableTerminalRuntime, RuntimeSettings};
use terminal_presentation::handlers::TerminalHandlers;

use crate::bridges::{TerminalEvents, TerminalLauncher};
use crate::config::Config;

/// The terminal module, wired: the PTY runtime, the `terminal.*` handlers,
/// the launcher that applies the login environment, and the plain events.
pub struct TerminalServices {
    pub runtime: Arc<dyn TerminalRuntime>,
    pub handlers: Arc<TerminalHandlers>,
    pub launcher: TerminalLauncher,
    pub events: TerminalEvents,
}

/// The environment a launched agent starts from: the server's own, and the
/// login shell's. Tests give stand-ins.
pub struct LoginSetup {
    pub login_environment: Arc<LoginEnvironment>,
    pub server_environment: Environment,
}

impl LoginSetup {
    /// The real login shell of the user and the process environment.
    #[must_use]
    pub fn detect() -> Self {
        let server_environment = process_environment();
        Self {
            login_environment: Arc::new(LoginEnvironment::detect(&server_environment)),
            server_environment,
        }
    }
}

/// Builds the terminal services from the settings and the environment setup.
///
/// # Errors
/// A setting that is not a valid output limit.
pub fn build_terminal_services(
    config: &Config,
    settings: &Settings,
    login: LoginSetup,
) -> Result<TerminalServices, TerminalError> {
    let line_limit = usize::try_from(settings.output_line_limit).unwrap_or(usize::MAX);
    let events = TerminalEvents::new();
    let runtime: Arc<dyn TerminalRuntime> = Arc::new(PortableTerminalRuntime::new(
        RuntimeSettings {
            output_folder: config.output_folder_path(),
            limits: OutputLimits::with_lines(line_limit)?,
            quiet_period: Duration::from_millis(settings.quiet_period_ms),
        },
        Arc::new(events.clone()) as Arc<dyn TerminalEventSink>,
    ));
    Ok(TerminalServices {
        handlers: Arc::new(TerminalHandlers::new(&runtime)),
        launcher: TerminalLauncher::new(
            runtime.clone(),
            login.login_environment,
            login.server_environment,
        ),
        runtime,
        events,
    })
}

/// The server's own environment, without the variables that are not text.
fn process_environment() -> Environment {
    std::env::vars_os()
        .filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)))
        .collect()
}
