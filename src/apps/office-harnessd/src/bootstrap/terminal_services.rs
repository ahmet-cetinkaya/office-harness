// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::time::Duration;

use agent_application::common::interfaces::{FileView, WorktreeManager};
use agent_domain::value_objects::Settings;
use agent_infrastructure::services::{
    AttributionTiming, Environment, GitWorktrees, LazyGitRunner, LocalFileView, LoginEnvironment,
};
use kernel::TextRedactor;
use terminal_application::common::interfaces::{
    ProcessInspector, TerminalEventSink, TerminalRuntime,
};
use terminal_domain::errors::TerminalError;
use terminal_domain::value_objects::OutputLimits;
use terminal_infrastructure::services::{
    PortableTerminalRuntime, RuntimeSettings, SystemProcessInspector,
};
use terminal_presentation::handlers::TerminalHandlers;

use super::StorageAlerts;
use crate::bridges::{
    HookSessions, PersonInput, SignInEnds, SignInWatchTiming, TerminalEvents, TerminalLauncher,
    WaitTracker,
};
use crate::config::Config;

/// The terminal module, wired: the PTY runtime, the `terminal.*` handlers,
/// the launcher that applies the login environment, and the plain events.
pub struct TerminalServices {
    pub runtime: Arc<dyn TerminalRuntime>,
    /// Replaces secret values in what the office stores of agent output and
    /// processes (0026/FR-012); the stores of other modules take it from here.
    pub redactor: Arc<dyn TextRedactor>,
    /// Reads the processes of this machine, for the identities of 0017.
    pub inspector: Arc<dyn ProcessInspector>,
    pub handlers: Arc<TerminalHandlers>,
    pub launcher: TerminalLauncher,
    pub events: TerminalEvents,
    /// The agents' own git worktrees (0010). It runs git with the same login
    /// environment the agents start with.
    pub worktrees: Arc<dyn WorktreeManager>,
    /// The read-only view of an agent's folder (0016). It shares the git
    /// runner of the worktrees.
    pub file_view: Arc<dyn FileView>,
    /// What people type into each terminal, for the directive gate (0012).
    pub person_input: PersonInput,
    /// What the reports of `claude` sessions say of each running session
    /// (0024). It shares the wait tracker of `person_input`.
    pub hook_sessions: HookSessions,
    /// The runs that the sign-in watch stopped, for the status bridge (0019).
    pub sign_in_ends: SignInEnds,
    pub sign_in_timing: SignInWatchTiming,
}

/// The environment a launched agent starts from: the server's own, and the
/// login shell's, and how long to look for the session id of a CLI that does
/// not take one. Tests give stand-ins.
pub struct LoginSetup {
    pub login_environment: Arc<LoginEnvironment>,
    pub server_environment: Environment,
    pub attribution: AttributionTiming,
    /// How long each new session is watched for a sign-in screen (0019).
    pub sign_in_watch: SignInWatchTiming,
}

impl LoginSetup {
    /// The real login shell of the user and the process environment.
    #[must_use]
    pub fn detect() -> Self {
        let server_environment = process_environment();
        Self {
            login_environment: Arc::new(LoginEnvironment::detect(&server_environment)),
            server_environment,
            attribution: AttributionTiming::default(),
            sign_in_watch: SignInWatchTiming::default(),
        }
    }
}

/// Builds the terminal services from the settings and the environment setup.
/// What an undone worktree create leaves behind goes to `alerts`.
///
/// # Errors
/// A setting that is not a valid output limit.
pub fn build_terminal_services(
    config: &Config,
    settings: &Settings,
    login: LoginSetup,
    alerts: &StorageAlerts,
    redactor: Arc<dyn TextRedactor>,
) -> Result<TerminalServices, TerminalError> {
    let LoginSetup {
        login_environment,
        server_environment,
        sign_in_watch,
        ..
    } = login;
    let quiet_period = Duration::from_millis(settings.quiet_period_ms);
    let line_limit = usize::try_from(settings.output_line_limit).unwrap_or(usize::MAX);
    let events = TerminalEvents::new();
    let runtime: Arc<dyn TerminalRuntime> = Arc::new(
        PortableTerminalRuntime::new(
            RuntimeSettings {
                output_folder: config.output_folder_path(),
                limits: OutputLimits::with_lines(line_limit)?,
                quiet_period,
            },
            Arc::new(events.clone()) as Arc<dyn TerminalEventSink>,
        )
        .with_redactor(Arc::clone(&redactor)),
    );
    let git = Arc::new(LazyGitRunner::login(
        config.worktrees_folder_path(),
        Arc::clone(&login_environment),
        server_environment.clone(),
    ));
    let worktrees = Arc::new(GitWorktrees::new(
        config.worktrees_folder_path(),
        Arc::clone(&git),
        Arc::new(alerts.clone()),
    ));
    let waits = WaitTracker::new(quiet_period);
    Ok(TerminalServices {
        handlers: Arc::new(TerminalHandlers::new(&runtime)),
        launcher: TerminalLauncher::new(runtime.clone(), login_environment, server_environment),
        worktrees,
        inspector: Arc::new(SystemProcessInspector::new()),
        file_view: Arc::new(LocalFileView::new(git)),
        runtime,
        redactor,
        events,
        person_input: PersonInput::default().with_wait_tracker(waits.clone()),
        hook_sessions: HookSessions::new(waits),
        sign_in_ends: SignInEnds::default(),
        sign_in_timing: sign_in_watch,
    })
}

/// The server's own environment, without the variables that are not text.
fn process_environment() -> Environment {
    std::env::vars_os()
        .filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)))
        .collect()
}
