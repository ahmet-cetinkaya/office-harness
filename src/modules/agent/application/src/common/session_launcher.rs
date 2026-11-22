// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_domain::entities::{Agent, Session};
use agent_domain::enums::StatusEvent;
use agent_domain::errors::AgentError;
use agent_domain::value_objects::{ExitResult, OfficeAdditions, SessionId};

use crate::common::command_line::split_command;
use crate::common::continuation::adapter_of;
use crate::common::errors::{ApplicationError, PortError, TerminalHostError};
use crate::common::interfaces::{
    AdditionsStore, AgentStore, CliAdapter, CliAdapters, Clock, FailureReporter, ProjectStore,
    SessionStore, SessionTracking, TerminalHost,
};
use crate::common::lookup::require_project;
use crate::common::models::{
    PrepareLaunch, PreparedLaunch, SessionCommand, SessionPlan, StartTerminal, TrackedSession,
    TrackingRequest,
};
use crate::common::worktree_link::is_linked_worktree;

/// What a start was asked to do.
#[derive(Debug, Clone, Copy)]
pub enum LaunchKind<'a> {
    /// A new CLI session (FR-024).
    NewTask,
    /// Continues `from`, whose CLI session id the caller checked (FR-025).
    Continue {
        from: &'a Session,
        cli_session_id: &'a str,
    },
}

/// The agent after a start that was accepted, and its new session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartOutcome {
    /// `starting`, or `failed` when the command could not start.
    pub agent: Agent,
    pub session: Session,
    /// The text to show when the command could not start.
    pub failure: Option<String>,
}

/// Starts one session of an agent's CLI: finds the program, builds the
/// command line with the adapter, records the session with the additions it
/// used (FR-023), and starts the terminal. A command that cannot start fails
/// this agent only; it is not an error of the request.
#[derive(Clone)]
pub struct SessionLauncher {
    agents: Arc<dyn AgentStore>,
    projects: Arc<dyn ProjectStore>,
    additions: Arc<dyn AdditionsStore>,
    sessions: Arc<dyn SessionStore>,
    terminals: Arc<dyn TerminalHost>,
    adapters: Arc<dyn CliAdapters>,
    clock: Arc<dyn Clock>,
    failures: Arc<dyn FailureReporter>,
}

/// What a launcher needs besides the stores.
pub struct LauncherServices {
    pub terminals: Arc<dyn TerminalHost>,
    pub adapters: Arc<dyn CliAdapters>,
    pub clock: Arc<dyn Clock>,
    pub failures: Arc<dyn FailureReporter>,
}

/// A start that `prepare` accepted: the new session's id, the additions it
/// will use, and the command to run.
pub struct PreparedStart {
    session_id: SessionId,
    additions: OfficeAdditions,
    readiness: Readiness,
}

enum Readiness {
    Ready(Prepared),
    /// The program cannot start: the start stores a session that never ran.
    NotStartable(String),
}

struct Prepared {
    folder: PathBuf,
    launch: PreparedLaunch,
    command: SessionCommand,
    adapter: Arc<dyn CliAdapter>,
}

impl SessionLauncher {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        projects: Arc<dyn ProjectStore>,
        additions: Arc<dyn AdditionsStore>,
        sessions: Arc<dyn SessionStore>,
        services: LauncherServices,
    ) -> Self {
        Self {
            agents,
            projects,
            additions,
            sessions,
            terminals: services.terminals,
            adapters: services.adapters,
            clock: services.clock,
            failures: services.failures,
        }
    }

    /// Checks everything that can refuse the start, and builds the command,
    /// without changing anything. A caller that ends a running session first
    /// calls this before it, so a refusal loses nothing.
    ///
    /// # Errors
    /// `Invalid` naming the missing folder, the unsafe argument, or the bad
    /// command; `Refused` when the CLI cannot continue a session.
    pub fn prepare(
        &self,
        agent: &Agent,
        kind: &LaunchKind<'_>,
    ) -> Result<PreparedStart, ApplicationError> {
        let folder = self.existing_working_folder(agent)?;
        let additions = self.used_additions(agent, kind)?;
        let session_id = SessionId::generate();
        let readiness = match self.build_launch(agent, &folder, &additions, session_id, kind) {
            Ok(prepared) => Readiness::Ready(prepared),
            Err(PrepareError::NotStartable(text)) => Readiness::NotStartable(text),
            Err(PrepareError::Refused(error)) => return Err(error),
        };
        Ok(PreparedStart {
            session_id,
            additions,
            readiness,
        })
    }

    /// Prepares and starts in one step, for an agent that does not run.
    ///
    /// # Errors
    /// As `prepare`, and `Conflict` when the agent already runs. A refusal
    /// changes no stored status.
    pub fn launch(
        &self,
        agent: &mut Agent,
        kind: &LaunchKind<'_>,
    ) -> Result<StartOutcome, ApplicationError> {
        let prepared = self.prepare(agent, kind)?;
        self.start(agent, kind, prepared)
    }

    /// Starts what `prepare` built. The agent must not run any more.
    ///
    /// # Errors
    /// `Conflict` when the agent already runs, and the errors of the stores.
    pub fn start(
        &self,
        agent: &mut Agent,
        kind: &LaunchKind<'_>,
        prepared: PreparedStart,
    ) -> Result<StartOutcome, ApplicationError> {
        agent.apply(StatusEvent::Start).map_err(|_| {
            ApplicationError::Conflict(format!("agent {} is already running", agent.name))
        })?;
        let PreparedStart {
            session_id,
            additions,
            readiness,
        } = prepared;
        let launch = match readiness {
            Readiness::Ready(launch) => launch,
            Readiness::NotStartable(text) => {
                let session = begin_session(agent, session_id, None, kind, additions, self.now());
                return self.fail_never_started(agent, session, text);
            }
        };
        let cli_session_id = match kind {
            LaunchKind::NewTask => launch.command.cli_session_id.clone(),
            LaunchKind::Continue { cli_session_id, .. } => Some((*cli_session_id).to_owned()),
        };
        // Taken after the running session ended, so its files are not new.
        let tracking = launch.adapter.track_new_session(&TrackingRequest {
            working_folder: &launch.folder,
            environment: &launch.launch.environment,
        });
        let session = begin_session(
            agent,
            session_id,
            cli_session_id,
            kind,
            additions,
            self.now(),
        );
        self.begin_terminal(agent, kind, session, &launch, tracking)
    }

    fn build_launch(
        &self,
        agent: &Agent,
        folder: &Path,
        additions: &OfficeAdditions,
        session_id: SessionId,
        kind: &LaunchKind<'_>,
    ) -> Result<Prepared, PrepareError> {
        let (program, fixed_arguments) =
            split_command(&agent.command).map_err(PrepareError::Refused)?;
        let launch = self
            .terminals
            .prepare(&PrepareLaunch {
                program: program.clone(),
                working_folder: folder.to_path_buf(),
                environment_additions: additions
                    .env()
                    .iter()
                    .filter_map(|entry| {
                        entry
                            .value()
                            .map(|value| (entry.name().to_owned(), value.to_owned()))
                    })
                    .collect(),
                agent_id: agent.id,
                session_id,
            })
            .map_err(|error| match error {
                TerminalHostError::Failed(text) => PrepareError::NotStartable(text),
                other => PrepareError::Refused(host_failure(other)),
            })?;
        let adapter = adapter_of(&*self.adapters, &agent.command).map_err(PrepareError::Refused)?;
        let proposed_cli_session_id = session_id.to_string();
        let plan = SessionPlan {
            fixed_arguments: &fixed_arguments,
            flags: additions.flags(),
            startup_prompt: additions.startup_prompt(),
            proposed_cli_session_id: &proposed_cli_session_id,
        };
        let command = match kind {
            LaunchKind::NewTask => adapter.new_session(&plan),
            LaunchKind::Continue { cli_session_id, .. } => adapter
                .continue_session(&plan, cli_session_id)
                .ok_or_else(|| {
                    PrepareError::Refused(ApplicationError::Refused(
                        "the agent's command does not support continuing a session".to_owned(),
                    ))
                })?,
        };
        self.terminals
            .validate_arguments(&launch, &command.arguments)
            .map_err(|error| PrepareError::Refused(host_failure(error)))?;
        Ok(Prepared {
            folder: folder.to_path_buf(),
            launch,
            command,
            adapter,
        })
    }

    fn begin_terminal(
        &self,
        agent: &mut Agent,
        kind: &LaunchKind<'_>,
        session: Session,
        prepared: &Prepared,
        tracking: Option<Box<dyn SessionTracking>>,
    ) -> Result<StartOutcome, ApplicationError> {
        self.sessions.insert(&session)?;
        self.agents.update_status(agent.id, agent.status)?;
        let started = self.terminals.start(&StartTerminal {
            agent_id: agent.id,
            session_id: session.id,
            program: prepared.launch.program.clone(),
            arguments: prepared.command.arguments.clone(),
            working_folder: prepared.folder.clone(),
            environment: prepared.launch.environment.clone(),
            typed_input: prepared.command.typed_input.clone(),
        });
        if let Err(error) = started {
            return self.fail_after_start(agent, session, error.to_string());
        }
        if let Some(tracking) = tracking {
            match kind {
                LaunchKind::NewTask => self.follow(session.id, tracking),
                LaunchKind::Continue { .. } => Self::hold_attribution_window(tracking),
            }
        }
        Ok(StartOutcome {
            agent: agent.clone(),
            session,
            failure: None,
        })
    }

    /// Learns the CLI's session id in the background. A session that is not
    /// matched keeps no id, so it cannot be continued, which is safe.
    fn follow(&self, session_id: SessionId, tracking: Box<dyn SessionTracking>) {
        let sessions = Arc::clone(&self.sessions);
        let failures = Arc::clone(&self.failures);
        tracking.follow(Box::new(move |tracked| {
            let TrackedSession::Found(cli_session_id) = tracked else {
                return;
            };
            match sessions.set_cli_session_id(session_id, &cli_session_id) {
                // A deleted agent takes its sessions along, so nothing is
                // left to report to.
                Ok(()) | Err(PortError::NotFound { .. }) => {}
                Err(error) => failures.report(&error),
            }
        }));
    }

    /// A continued session keeps the id it has. The tracking still runs, so
    /// that a new task that starts in the same folder meanwhile is not
    /// matched to a file this start may create.
    fn hold_attribution_window(tracking: Box<dyn SessionTracking>) {
        tracking.follow(Box::new(|_| {}));
    }

    /// The command could not be found: the session is stored as one that
    /// never started.
    fn fail_never_started(
        &self,
        agent: &mut Agent,
        mut session: Session,
        text: String,
    ) -> Result<StartOutcome, ApplicationError> {
        session.end(ExitResult::SpawnError(text.clone()), self.now());
        self.sessions.insert(&session)?;
        self.fail_agent(agent, session, text)
    }

    /// The terminal refused to start the stored session.
    fn fail_after_start(
        &self,
        agent: &mut Agent,
        mut session: Session,
        text: String,
    ) -> Result<StartOutcome, ApplicationError> {
        let result = ExitResult::SpawnError(text.clone());
        self.sessions.end_running(agent.id, &result, self.now())?;
        session.end(result, self.now());
        self.fail_agent(agent, session, text)
    }

    fn fail_agent(
        &self,
        agent: &mut Agent,
        session: Session,
        text: String,
    ) -> Result<StartOutcome, ApplicationError> {
        agent.apply(StatusEvent::ExitedWithFailure)?;
        self.agents.update_status(agent.id, agent.status)?;
        Ok(StartOutcome {
            agent: agent.clone(),
            session,
            failure: Some(text),
        })
    }

    /// The additions a session uses: the stored ones, and for a continue
    /// without the startup prompt, which goes only to a new session.
    fn used_additions(
        &self,
        agent: &Agent,
        kind: &LaunchKind<'_>,
    ) -> Result<OfficeAdditions, ApplicationError> {
        let stored = self
            .additions
            .load(agent.id)?
            .map_or_default(|record| record.additions);
        match kind {
            LaunchKind::NewTask => Ok(stored),
            LaunchKind::Continue { .. } => Ok(OfficeAdditions::new(
                stored.flags().to_vec(),
                stored.env().to_vec(),
                None,
            )?),
        }
    }

    fn existing_working_folder(&self, agent: &Agent) -> Result<PathBuf, ApplicationError> {
        if let Some(worktree) = &agent.worktree {
            require_linked_worktree(&worktree.folder)?;
        }
        let (field, folder) = match (&agent.working_folder, agent.project_id) {
            (Some(folder), _) => ("working folder", folder.clone()),
            (None, Some(project_id)) => (
                "project folder",
                require_project(&*self.projects, project_id)?.folder,
            ),
            (None, None) => return Err(AgentError::NoWorkingFolderOrProject.into()),
        };
        if folder.is_dir() {
            return Ok(folder);
        }
        Err(ApplicationError::Invalid(format!(
            "the {field} does not exist: {}",
            folder.display()
        )))
    }

    fn now(&self) -> kernel::Timestamp {
        self.clock.now()
    }
}

/// Refuses the start of an agent whose worktree is gone or no longer linked
/// to its repository (0010 FR-008).
fn require_linked_worktree(folder: &Path) -> Result<(), ApplicationError> {
    if is_linked_worktree(folder) {
        return Ok(());
    }
    Err(ApplicationError::Invalid(format!(
        "the worktree folder is missing or broken: {}",
        folder.display()
    )))
}

enum PrepareError {
    /// The program cannot start: the agent fails, and the request succeeds.
    NotStartable(String),
    /// The request is refused and nothing changes.
    Refused(ApplicationError),
}

fn host_failure(error: TerminalHostError) -> ApplicationError {
    match error {
        TerminalHostError::Refused(text) => ApplicationError::Invalid(text),
        other => ApplicationError::Internal(other.to_string()),
    }
}

fn begin_session(
    agent: &Agent,
    id: SessionId,
    cli_session_id: Option<String>,
    kind: &LaunchKind<'_>,
    additions: OfficeAdditions,
    started_at: kernel::Timestamp,
) -> Session {
    let continued_from = match kind {
        LaunchKind::NewTask => None,
        LaunchKind::Continue { from, .. } => Some(from.id),
    };
    Session::begin(
        id,
        agent.id,
        cli_session_id,
        continued_from,
        additions,
        started_at,
    )
}
