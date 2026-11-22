// SPDX-License-Identifier: AGPL-3.0-or-later

//! In-memory fakes of the ports, shared by the unit tests.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use agent_domain::entities::{AdditionsRecord, Agent, Project, Session};
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{
    AgentCommand, AgentWorktree, DisplayName, ExitResult, MachineId, ProjectId, SessionId,
    Settings, TaskText,
};
use kernel::{AgentId, Timestamp};

use crate::commands::continue_session::ContinueSessionHandler;
use crate::commands::create_agent::CreateAgentHandler;
use crate::commands::delete_agent::DeleteAgentHandler;
use crate::commands::end_session::EndSessionHandler;
use crate::commands::resume_agent::ResumeAgentHandler;
use crate::commands::set_office_additions::SetOfficeAdditionsHandler;
use crate::commands::set_task_text::SetTaskTextHandler;
use crate::commands::start_agent::StartAgentHandler;
use crate::commands::start_new_task::StartNewTaskHandler;
use crate::commands::stop_agent::StopAgentHandler;
use crate::common::action_journal::ActionJournal;
use crate::common::errors::{PortError, TerminalHostError};
use crate::common::interfaces::{
    ActionRecorder, AdditionsStore, AgentStore, CliAdapter, CliAdapters, Clock, CreatedWorktree,
    FailureReporter, ProjectStore, SessionStore, SessionTracking, SettingsStore, TerminalHost,
    WorktreeCreate, WorktreeError, WorktreeInfo, WorktreeManager, WorktreeStatus,
};
use crate::common::models::{
    AgentActionEntry, PrepareLaunch, PreparedLaunch, SessionCommand, SessionPlan, StartTerminal,
    TrackedSession, TrackingRequest,
};
use crate::common::session_ender::SessionEnder;
use crate::common::session_launcher::{LauncherServices, SessionLauncher};
use crate::queries::get_office_additions::GetOfficeAdditionsHandler;
use crate::queries::list_sessions::ListSessionsHandler;

const NOW_MILLIS: i64 = 1_700_000_000_000;

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[must_use]
pub fn absolute_folder(name: &str) -> PathBuf {
    std::env::temp_dir().join(name)
}

#[must_use]
pub fn project_named(name: &str) -> Project {
    Project::new(
        ProjectId::generate(),
        DisplayName::parse("project name", name).expect("name"),
        absolute_folder("project"),
    )
    .expect("project")
}

#[must_use]
pub fn agent_in_project(name: &str, project_id: ProjectId) -> Agent {
    Agent::new(
        AgentId::generate(),
        DisplayName::parse("agent name", name).expect("name"),
        AgentCommand::parse("claude").expect("command"),
        None,
        Some(project_id),
        MachineId::generate(),
        Timestamp::from_unix_millis(NOW_MILLIS),
    )
    .expect("agent")
}

#[derive(Default)]
pub struct MemoryAgents(Mutex<Vec<Agent>>);

impl MemoryAgents {
    #[must_use]
    pub fn all(&self) -> Vec<Agent> {
        locked(&self.0).clone()
    }

    #[must_use]
    pub fn status_of(&self, id: AgentId) -> Option<AgentStatus> {
        locked(&self.0)
            .iter()
            .find(|agent| agent.id == id)
            .map(|agent| agent.status)
    }
}

impl AgentStore for MemoryAgents {
    fn insert(&self, agent: &Agent) -> Result<(), PortError> {
        let mut agents = locked(&self.0);
        let key = agent.name.uniqueness_key();
        if agents
            .iter()
            .any(|other| other.name.uniqueness_key() == key)
        {
            return Err(PortError::Duplicate {
                what: format!("agent {}", agent.name),
            });
        }
        agents.push(agent.clone());
        Ok(())
    }

    fn find(&self, id: AgentId) -> Result<Option<Agent>, PortError> {
        Ok(locked(&self.0).iter().find(|agent| agent.id == id).cloned())
    }

    fn list(&self) -> Result<Vec<Agent>, PortError> {
        Ok(self.all())
    }

    fn update_status(&self, id: AgentId, status: AgentStatus) -> Result<(), PortError> {
        let mut agents = locked(&self.0);
        let agent = agents
            .iter_mut()
            .find(|agent| agent.id == id)
            .ok_or_else(|| PortError::NotFound {
                what: format!("agent {id}"),
            })?;
        agent.status = status;
        Ok(())
    }

    fn update_task(&self, id: AgentId, task: Option<&TaskText>) -> Result<(), PortError> {
        let mut agents = locked(&self.0);
        let agent = agents
            .iter_mut()
            .find(|agent| agent.id == id)
            .ok_or_else(|| PortError::NotFound {
                what: format!("agent {id}"),
            })?;
        agent.task = task.cloned();
        Ok(())
    }

    fn delete(&self, id: AgentId) -> Result<(), PortError> {
        let mut agents = locked(&self.0);
        let before = agents.len();
        agents.retain(|agent| agent.id != id);
        if agents.len() == before {
            return Err(PortError::NotFound {
                what: format!("agent {id}"),
            });
        }
        Ok(())
    }
}

pub struct MemoryProjects {
    projects: Mutex<Vec<Project>>,
    agents: Arc<MemoryAgents>,
}

impl MemoryProjects {
    #[must_use]
    pub fn new(agents: Arc<MemoryAgents>) -> Self {
        Self {
            projects: Mutex::new(Vec::new()),
            agents,
        }
    }

    #[must_use]
    pub fn all(&self) -> Vec<Project> {
        locked(&self.projects).clone()
    }
}

impl ProjectStore for MemoryProjects {
    fn insert(&self, project: &Project) -> Result<(), PortError> {
        let mut projects = locked(&self.projects);
        let key = project.name.uniqueness_key();
        if projects
            .iter()
            .any(|other| other.name.uniqueness_key() == key)
        {
            return Err(PortError::Duplicate {
                what: format!("project {}", project.name),
            });
        }
        projects.push(project.clone());
        Ok(())
    }

    fn find(&self, id: ProjectId) -> Result<Option<Project>, PortError> {
        Ok(locked(&self.projects)
            .iter()
            .find(|project| project.id == id)
            .cloned())
    }

    fn list(&self) -> Result<Vec<Project>, PortError> {
        Ok(self.all())
    }

    fn delete(&self, id: ProjectId) -> Result<(), PortError> {
        let users: Vec<String> = self
            .agents
            .all()
            .iter()
            .filter(|agent| agent.project_id == Some(id))
            .map(|agent| agent.name.to_string())
            .collect();
        let mut projects = locked(&self.projects);
        let name = projects
            .iter()
            .find(|project| project.id == id)
            .map(|project| project.name.to_string())
            .ok_or_else(|| PortError::NotFound {
                what: format!("project {id}"),
            })?;
        if !users.is_empty() {
            return Err(PortError::ProjectInUse {
                project: name,
                agents: users,
            });
        }
        projects.retain(|project| project.id != id);
        Ok(())
    }
}

#[derive(Default)]
pub struct MemorySettings(Mutex<Settings>);

impl MemorySettings {
    #[must_use]
    pub fn current(&self) -> Settings {
        locked(&self.0).clone()
    }
}

impl SettingsStore for MemorySettings {
    fn load(&self) -> Result<Settings, PortError> {
        Ok(self.current())
    }

    fn save(&self, settings: &Settings) -> Result<(), PortError> {
        locked(&self.0).clone_from(settings);
        Ok(())
    }
}

/// Sessions in memory.
#[derive(Default)]
pub struct MemorySessions(Mutex<Vec<Session>>);

impl MemorySessions {
    #[must_use]
    pub fn all(&self) -> Vec<Session> {
        locked(&self.0).clone()
    }
}

impl SessionStore for MemorySessions {
    fn insert(&self, session: &Session) -> Result<(), PortError> {
        locked(&self.0).push(session.clone());
        Ok(())
    }

    fn find(&self, id: SessionId) -> Result<Option<Session>, PortError> {
        Ok(locked(&self.0)
            .iter()
            .find(|session| session.id == id)
            .cloned())
    }

    fn list_for_agent(&self, agent_id: AgentId) -> Result<Vec<Session>, PortError> {
        let mut sessions: Vec<Session> = locked(&self.0)
            .iter()
            .filter(|session| session.agent_id == agent_id)
            .cloned()
            .collect();
        sessions.reverse();
        Ok(sessions)
    }

    fn latest_for_agent(&self, agent_id: AgentId) -> Result<Option<Session>, PortError> {
        Ok(self.list_for_agent(agent_id)?.into_iter().next())
    }

    fn set_cli_session_id(&self, id: SessionId, cli_session_id: &str) -> Result<(), PortError> {
        let mut sessions = locked(&self.0);
        let session = sessions
            .iter_mut()
            .find(|session| session.id == id)
            .ok_or_else(|| PortError::NotFound {
                what: format!("session {id}"),
            })?;
        session.cli_session_id = Some(cli_session_id.to_owned());
        Ok(())
    }

    fn end_running(
        &self,
        agent_id: AgentId,
        result: &ExitResult,
        at: Timestamp,
    ) -> Result<bool, PortError> {
        let mut sessions = locked(&self.0);
        let running = sessions
            .iter_mut()
            .find(|session| session.agent_id == agent_id && session.is_running());
        Ok(running
            .map(|session| session.end(result.clone(), at))
            .is_some())
    }
}

#[derive(Default)]
pub struct MemoryAdditions(Mutex<Vec<(AgentId, AdditionsRecord)>>);

impl AdditionsStore for MemoryAdditions {
    fn load(&self, agent_id: AgentId) -> Result<Option<AdditionsRecord>, PortError> {
        Ok(locked(&self.0)
            .iter()
            .find(|(id, _)| *id == agent_id)
            .map(|(_, record)| record.clone()))
    }

    fn save(&self, agent_id: AgentId, record: &AdditionsRecord) -> Result<(), PortError> {
        let mut rows = locked(&self.0);
        rows.retain(|(id, _)| *id != agent_id);
        rows.push((agent_id, record.clone()));
        Ok(())
    }
}

/// Reports nothing: the tests that need no background failure.
struct IgnoredFailures;

impl FailureReporter for IgnoredFailures {
    fn report(&self, _error: &PortError) {}
}

/// Adapters that build recognizable command lines. A program named `generic`
/// cannot continue, and a program named `tracked` learns its id after the
/// start, as `codex` does.
pub struct FakeAdapters;

struct FakeAdapter {
    supports_continue: bool,
    tracked_id: Option<&'static str>,
}

struct ImmediateTracking(&'static str);

impl SessionTracking for ImmediateTracking {
    fn follow(self: Box<Self>, report: Box<dyn FnOnce(TrackedSession) + Send>) {
        report(TrackedSession::Found(self.0.to_owned()));
    }
}

fn command_line(plan: &SessionPlan<'_>, middle: &[&str], with_prompt: bool) -> SessionCommand {
    let mut arguments: Vec<String> = plan.fixed_arguments.to_vec();
    arguments.extend(plan.flags.iter().cloned());
    arguments.extend(middle.iter().map(|word| (*word).to_owned()));
    let typed_input = plan
        .startup_prompt
        .filter(|_| with_prompt)
        .map(str::to_owned);
    SessionCommand {
        arguments,
        cli_session_id: None,
        typed_input,
    }
}

impl CliAdapter for FakeAdapter {
    fn supports_continue(&self) -> bool {
        self.supports_continue
    }

    fn new_session(&self, plan: &SessionPlan<'_>) -> SessionCommand {
        if self.tracked_id.is_some() {
            return command_line(plan, &[], true);
        }
        let mut command = command_line(plan, &["--session-id", plan.proposed_cli_session_id], true);
        command.cli_session_id = Some(plan.proposed_cli_session_id.to_owned());
        command
    }

    fn continue_session(
        &self,
        plan: &SessionPlan<'_>,
        cli_session_id: &str,
    ) -> Option<SessionCommand> {
        self.supports_continue
            .then(|| command_line(plan, &["--resume", cli_session_id], false))
    }

    fn track_new_session(
        &self,
        _request: &TrackingRequest<'_>,
    ) -> Option<Box<dyn SessionTracking>> {
        self.tracked_id
            .map(|id| Box::new(ImmediateTracking(id)) as Box<dyn SessionTracking>)
    }
}

impl CliAdapters for FakeAdapters {
    fn for_program(&self, program: &str) -> Arc<dyn CliAdapter> {
        Arc::new(FakeAdapter {
            supports_continue: program != "generic",
            tracked_id: (program == "tracked").then_some("tracked-id"),
        })
    }
}

pub struct FixedClock;

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_millis(NOW_MILLIS)
    }
}

/// Keeps every entry; after `fail_from_now` it refuses every write.
#[derive(Default)]
pub struct RecordingRecorder {
    entries: Mutex<Vec<AgentActionEntry>>,
    is_failing: Mutex<bool>,
}

impl RecordingRecorder {
    #[must_use]
    pub fn entries(&self) -> Vec<AgentActionEntry> {
        locked(&self.entries).clone()
    }

    pub fn fail_from_now(&self) {
        *locked(&self.is_failing) = true;
    }
}

impl ActionRecorder for RecordingRecorder {
    fn record(&self, entry: &AgentActionEntry) -> Result<(), PortError> {
        if *locked(&self.is_failing) {
            return Err(PortError::Storage {
                message: "the record is full".to_owned(),
                is_disk_failure: true,
            });
        }
        locked(&self.entries).push(entry.clone());
        Ok(())
    }
}

/// A terminal host that starts nothing: it keeps the requests, and a command
/// whose program is in `unstartable` fails with a text.
#[derive(Default)]
pub struct FakeTerminals {
    started: Mutex<Vec<StartTerminal>>,
    stopped: Mutex<Vec<(AgentId, Duration)>>,
    unstartable: Mutex<Vec<String>>,
    batch_programs: Mutex<Vec<String>>,
    ended_on_its_own: Mutex<bool>,
    ignores_stop_requests: Mutex<bool>,
}

impl FakeTerminals {
    #[must_use]
    pub fn started(&self) -> Vec<StartTerminal> {
        locked(&self.started).clone()
    }

    #[must_use]
    pub fn stopped(&self) -> Vec<(AgentId, Duration)> {
        locked(&self.stopped).clone()
    }

    /// From now on a stop finds no running terminal, as when the child
    /// exited just before the request.
    pub fn end_children_on_their_own(&self) {
        *locked(&self.ended_on_its_own) = true;
    }

    /// From now on a stop takes the whole grace period, as when the child
    /// ignores the request and is forced at the end.
    pub fn ignore_stop_requests(&self) {
        *locked(&self.ignores_stop_requests) = true;
    }

    pub fn make_unstartable(&self, program: &str) {
        locked(&self.unstartable).push(program.to_owned());
    }

    /// The program counts as a Windows batch script: arguments with `%` are
    /// refused.
    pub fn make_batch_script(&self, program: &str) {
        locked(&self.batch_programs).push(program.to_owned());
    }
}

impl TerminalHost for FakeTerminals {
    fn prepare(&self, request: &PrepareLaunch) -> Result<PreparedLaunch, TerminalHostError> {
        if locked(&self.unstartable).contains(&request.program) {
            return Err(TerminalHostError::Failed(format!(
                "the command {} was not found on the login PATH",
                request.program
            )));
        }
        Ok(PreparedLaunch {
            program: request.program.clone(),
            environment: request.environment_additions.clone(),
            is_batch_script: locked(&self.batch_programs).contains(&request.program),
        })
    }

    fn validate_arguments(
        &self,
        prepared: &PreparedLaunch,
        arguments: &[String],
    ) -> Result<(), TerminalHostError> {
        let unsafe_argument = arguments.iter().position(|argument| argument.contains('%'));
        match unsafe_argument {
            Some(index) if prepared.is_batch_script => Err(TerminalHostError::Refused(format!(
                "argument {index} contains '%', which a batch script cannot receive safely"
            ))),
            _ => Ok(()),
        }
    }

    fn start(&self, request: &StartTerminal) -> Result<(), TerminalHostError> {
        locked(&self.started).push(request.clone());
        Ok(())
    }

    fn stop(&self, agent_id: AgentId, grace: Duration) -> Result<(), TerminalHostError> {
        if *locked(&self.ended_on_its_own) {
            return Err(TerminalHostError::NotRunning);
        }
        if *locked(&self.ignores_stop_requests) {
            thread::sleep(grace);
        }
        locked(&self.stopped).push((agent_id, grace));
        Ok(())
    }
}

/// Every port as an in-memory fake, with the handlers built on them.
pub struct Office {
    pub agents: Arc<MemoryAgents>,
    pub projects: Arc<MemoryProjects>,
    pub sessions: Arc<MemorySessions>,
    pub additions: Arc<MemoryAdditions>,
    pub settings: Arc<MemorySettings>,
    pub recorder: Arc<RecordingRecorder>,
    pub terminals: Arc<FakeTerminals>,
    pub machine_id: MachineId,
}

impl Office {
    #[must_use]
    pub fn new() -> Self {
        let agents = Arc::new(MemoryAgents::default());
        Self {
            projects: Arc::new(MemoryProjects::new(agents.clone())),
            agents,
            sessions: Arc::new(MemorySessions::default()),
            additions: Arc::new(MemoryAdditions::default()),
            settings: Arc::new(MemorySettings::default()),
            recorder: Arc::new(RecordingRecorder::default()),
            terminals: Arc::new(FakeTerminals::default()),
            machine_id: MachineId::generate(),
        }
    }

    fn journal(&self) -> ActionJournal {
        ActionJournal::new(self.recorder.clone(), Arc::new(FixedClock))
    }

    /// The create use case with the fake worktree service attached.
    #[must_use]
    pub fn create_agent_with(&self, worktrees: &Arc<FakeWorktrees>) -> CreateAgentHandler {
        self.create_agent()
            .with_worktrees(Arc::clone(worktrees) as Arc<dyn WorktreeManager>)
    }

    /// The delete use case with the fake worktree service attached.
    #[must_use]
    pub fn delete_agent_with(&self, worktrees: &Arc<FakeWorktrees>) -> DeleteAgentHandler {
        self.delete_agent()
            .with_worktrees(Arc::clone(worktrees) as Arc<dyn WorktreeManager>)
    }

    #[must_use]
    pub fn create_agent(&self) -> CreateAgentHandler {
        CreateAgentHandler::new(
            self.agents.clone(),
            self.projects.clone(),
            self.additions.clone(),
            Arc::new(FixedClock),
            self.journal(),
            self.machine_id,
        )
    }

    #[must_use]
    pub fn clock() -> Arc<dyn Clock> {
        Arc::new(FixedClock)
    }

    fn ender(&self) -> SessionEnder {
        SessionEnder::new(
            self.agents.clone(),
            self.settings.clone(),
            self.sessions.clone(),
            self.terminals.clone(),
            Arc::new(FixedClock),
        )
    }

    fn launcher(&self) -> SessionLauncher {
        SessionLauncher::new(
            self.agents.clone(),
            self.projects.clone(),
            self.additions.clone(),
            self.sessions.clone(),
            LauncherServices {
                terminals: self.terminals.clone(),
                adapters: Arc::new(FakeAdapters),
                clock: Arc::new(FixedClock),
                failures: Arc::new(IgnoredFailures),
            },
        )
    }

    #[must_use]
    pub fn start_agent(&self) -> StartAgentHandler {
        StartAgentHandler::new(self.agents.clone(), self.launcher(), self.journal())
    }

    #[must_use]
    pub fn start_new_task(&self) -> StartNewTaskHandler {
        StartNewTaskHandler::new(
            self.agents.clone(),
            self.ender(),
            self.launcher(),
            self.journal(),
        )
    }

    #[must_use]
    pub fn continue_session(&self) -> ContinueSessionHandler {
        ContinueSessionHandler::new(
            self.agents.clone(),
            self.sessions.clone(),
            Arc::new(FakeAdapters),
            self.ender(),
            self.launcher(),
            self.journal(),
        )
    }

    #[must_use]
    pub fn resume_agent(&self) -> ResumeAgentHandler {
        ResumeAgentHandler::new(
            self.agents.clone(),
            self.sessions.clone(),
            Arc::new(FakeAdapters),
            self.launcher(),
            self.journal(),
        )
    }

    #[must_use]
    pub fn stop_agent(&self) -> StopAgentHandler {
        StopAgentHandler::new(self.agents.clone(), self.ender(), self.journal())
    }

    #[must_use]
    pub fn end_session(&self) -> EndSessionHandler {
        EndSessionHandler::new(self.sessions.clone(), Arc::new(FixedClock))
    }

    #[must_use]
    pub fn set_task_text(&self) -> SetTaskTextHandler {
        SetTaskTextHandler::new(self.agents.clone(), Arc::new(FixedClock), self.journal())
    }

    #[must_use]
    pub fn set_additions(&self) -> SetOfficeAdditionsHandler {
        SetOfficeAdditionsHandler::new(
            self.agents.clone(),
            self.additions.clone(),
            Arc::new(FixedClock),
        )
    }

    #[must_use]
    pub fn get_additions(&self) -> GetOfficeAdditionsHandler {
        GetOfficeAdditionsHandler::new(self.agents.clone(), self.additions.clone())
    }

    #[must_use]
    pub fn list_sessions(&self) -> ListSessionsHandler {
        ListSessionsHandler::new(
            self.agents.clone(),
            self.sessions.clone(),
            Arc::new(FakeAdapters),
        )
    }

    #[must_use]
    pub fn delete_agent(&self) -> DeleteAgentHandler {
        DeleteAgentHandler::new(
            self.agents.clone(),
            self.settings.clone(),
            self.terminals.clone(),
            self.journal(),
        )
    }

    /// Stores an agent with the command, folder, and status given.
    pub fn add_agent(
        &self,
        name: &str,
        command: &str,
        folder: &Path,
        status: AgentStatus,
    ) -> Agent {
        let mut agent = Agent::new(
            AgentId::generate(),
            DisplayName::parse("agent name", name).expect("name"),
            AgentCommand::parse(command).expect("command"),
            Some(folder.to_path_buf()),
            None,
            self.machine_id,
            Timestamp::from_unix_millis(NOW_MILLIS),
        )
        .expect("agent");
        agent.status = status;
        self.agents.insert(&agent).expect("insert");
        agent
    }
}

/// A clean, reachable worktree whose branch has no own commits.
#[must_use]
pub fn clean_status(hash: &str) -> WorktreeStatus {
    WorktreeStatus {
        folder_exists: true,
        reachable: true,
        changed_files: 0,
        status_hash: hash.to_owned(),
        own_commits: Some(0),
        base_exists: true,
        branch_commit: Some("c0".to_owned()),
    }
}

/// The git worktree service as a script: it records each call, answers the
/// statuses queued in `statuses` one by one (the last one repeats), and
/// fails what the test asks it to.
pub struct FakeWorktrees {
    pub folder: PathBuf,
    calls: Mutex<Vec<String>>,
    statuses: Mutex<Vec<WorktreeStatus>>,
    pub create_error: Mutex<Option<WorktreeError>>,
    pub remove_fails: Mutex<bool>,
}

impl FakeWorktrees {
    #[must_use]
    pub fn new(folder: &Path) -> Arc<Self> {
        Arc::new(Self {
            folder: folder.to_path_buf(),
            calls: Mutex::new(Vec::new()),
            statuses: Mutex::new(vec![clean_status("h0")]),
            create_error: Mutex::new(None),
            remove_fails: Mutex::new(false),
        })
    }

    #[must_use]
    pub fn calls(&self) -> Vec<String> {
        locked(&self.calls).clone()
    }

    pub fn queue_statuses(&self, statuses: Vec<WorktreeStatus>) {
        *locked(&self.statuses) = statuses;
    }

    fn record(&self, call: &str) {
        locked(&self.calls).push(call.to_owned());
    }

    /// The worktree the fake makes for an agent.
    #[must_use]
    pub fn worktree(&self) -> AgentWorktree {
        AgentWorktree {
            repository: absolute_folder("repo"),
            base: "main".to_owned(),
            branch: "office-harness/ana".to_owned(),
            folder: self.folder.clone(),
        }
    }
}

impl WorktreeManager for FakeWorktrees {
    fn info(&self, folder: &Path, agent_name: &str) -> Result<Option<WorktreeInfo>, WorktreeError> {
        self.record("info");
        Ok(Some(WorktreeInfo {
            top: folder.to_path_buf(),
            subfolder: String::new(),
            base: "main".to_owned(),
            base_is_branch: true,
            branches: vec!["main".to_owned()],
            branch_name: format!("office-harness/{}", agent_name.to_lowercase()),
        }))
    }

    fn create(&self, request: &WorktreeCreate<'_>) -> Result<CreatedWorktree, WorktreeError> {
        self.record(&format!("create {}", request.folder.display()));
        if let Some(error) = locked(&self.create_error).clone() {
            return Err(error);
        }
        Ok(CreatedWorktree {
            worktree: self.worktree(),
            working_folder: self.folder.join("sub"),
        })
    }

    fn status(&self, _worktree: &AgentWorktree) -> WorktreeStatus {
        self.record("status");
        let mut statuses = locked(&self.statuses);
        if statuses.len() > 1 {
            statuses.remove(0)
        } else {
            statuses[0].clone()
        }
    }

    fn remove(&self, _worktree: &AgentWorktree, force: bool) -> Result<(), WorktreeError> {
        self.record(if force { "remove --force" } else { "remove" });
        if *locked(&self.remove_fails) {
            return Err(WorktreeError::Failed("git refused".to_owned()));
        }
        Ok(())
    }

    fn delete_branch(&self, _worktree: &AgentWorktree, _commit: &str) -> Result<(), WorktreeError> {
        self.record("branch -D");
        Ok(())
    }

    fn discard(&self, _worktree: &AgentWorktree) {
        self.record("discard");
    }
}
