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

use super::hook_sessions::HookSessions;
use super::person_input::PersonInput;
use super::sign_in_watch::SignInWatch;
use super::status_hook_files::{SETTINGS_OPTION, StatusHookFiles};
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
    /// Told when a startup prompt waits for the person (0012 A11).
    person_input: PersonInput,
    /// Watches each new session for the CLI's sign-in screen (0019).
    sign_in_watch: Option<SignInWatch>,
    /// Told when a session starts, so that it is estimated until its first
    /// report (0024).
    hook_sessions: HookSessions,
    /// Makes and removes the office settings file of each `claude` session
    /// (0024).
    hook_files: StatusHookFiles,
}

/// What an agent session needs to call the office back: the store that
/// issues its credential and the helper that performs the call.
pub struct AgentCalls {
    pub store: Arc<dyn AgentCallStore>,
    pub helper: CallHelper,
    /// The office settings files and secrets of the status hooks (0024).
    pub hook_files: StatusHookFiles,
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
            person_input: PersonInput::default(),
            sign_in_watch: None,
            hook_sessions: HookSessions::default(),
            hook_files: agent_calls.hook_files,
        }
    }

    /// Watches the first seconds of each new session for the CLI's sign-in
    /// screen (0019/FR-005).
    #[must_use]
    pub fn with_sign_in_watch(mut self, watch: SignInWatch) -> Self {
        self.sign_in_watch = Some(watch);
        self
    }

    /// Shares the record of what the reports of each session say (0024).
    #[must_use]
    pub fn with_hook_sessions(mut self, hook_sessions: HookSessions) -> Self {
        self.hook_sessions = hook_sessions;
        self
    }

    /// Shares the record that a pasted startup prompt waits for the person,
    /// so directives hold back until the person types (0012 A11).
    #[must_use]
    pub fn with_person_input(mut self, person_input: PersonInput) -> Self {
        self.person_input = person_input;
        self
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
        let hooks = self.hook_files.plan(
            &request.status_hooks,
            &request.session_id.to_string(),
            &program,
            &prepared.environment,
        );
        Ok(PreparedLaunch {
            is_batch_script: is_batch_script(&program),
            program,
            environment: prepared.environment,
            hook_arguments: hooks.arguments,
            hook_note: Some(hooks.note),
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
        // A new session has an empty input line: nothing a person or the
        // office typed into an earlier one holds its directives (0012).
        self.person_input.session_started(request.agent_id);
        // Subscribed before the start, so the first output is not missed.
        let first_output = request
            .typed_input
            .as_ref()
            .map(|_| self.events.subscribe());
        let hooks = self.activate_hooks(request);
        self.launcher
            .spawn_prepared(SpawnRequest {
                agent_id: request.agent_id,
                program: request.program.clone(),
                arguments: hooks.arguments,
                working_folder: request.working_folder.clone(),
                environment: hooks.environment,
                size,
                output_key,
            })
            .inspect_err(|_| {
                self.calls
                    .revoke_session(request.agent_id, &request.session_id.to_string());
                self.hook_files.end(request.agent_id);
            })
            .map_err(|error| match error {
                LaunchError::Runtime(TerminalRuntimeError::AlreadyRunning(_)) => {
                    TerminalHostError::AlreadyRunning
                }
                other => failed(&other),
            })?;
        if let Some(watch) = &self.sign_in_watch {
            watch.start(
                request.agent_id,
                request.sign_in,
                request.typed_input.as_deref(),
            );
        }
        if let (Some(text), Some(events)) = (request.typed_input.clone(), first_output) {
            type_after_first_output(
                Arc::clone(&self.runtime),
                events,
                request.agent_id,
                text,
                self.log.clone(),
                self.person_input.clone(),
            );
        }
        let run = self.runtime.current_run(request.agent_id);
        self.person_input
            .wait_tracker()
            .session_started(request.agent_id, run);
        self.hook_sessions
            .session_started(request.agent_id, run, hooks.note);
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

/// What the spawn of one session gets of the office settings file (0024).
struct SpawnHooks {
    arguments: Vec<String>,
    environment: Vec<(String, String)>,
    /// The note that the hook state starts with.
    note: Option<String>,
}

impl TerminalHostBridge {
    /// Makes the file and the secret right before the spawn. A session whose
    /// file cannot be made runs without the option that names it.
    fn activate_hooks(&self, request: &StartTerminal) -> SpawnHooks {
        let mut hooks = SpawnHooks {
            arguments: request.arguments.clone(),
            environment: request.environment.clone(),
            note: request.hook_note.clone(),
        };
        if !request.has_hook_arguments {
            return hooks;
        }
        match self
            .hook_files
            .activate(request.agent_id, &request.session_id.to_string())
        {
            Ok(secret) => hooks.environment.push(secret),
            Err(reason) => {
                remove_settings_arguments(&mut hooks.arguments);
                hooks.note = Some(reason);
            }
        }
        hooks
    }
}

/// Removes the last `--settings <file>` pair, the one that the office added.
fn remove_settings_arguments(arguments: &mut Vec<String>) {
    if let Some(position) = arguments
        .iter()
        .rposition(|argument| argument == SETTINGS_OPTION)
    {
        let end = (position + 2).min(arguments.len());
        arguments.drain(position..end);
    }
}

fn refused(error: &TerminalError) -> TerminalHostError {
    TerminalHostError::Refused(error.to_string())
}
