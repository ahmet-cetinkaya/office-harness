// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, PoisonError};

use agent_application::commands::continue_session::ContinueSessionHandler;
use agent_application::commands::create_agent::CreateAgentHandler;
use agent_application::commands::create_project::CreateProjectHandler;
use agent_application::commands::delete_agent::DeleteAgentHandler;
use agent_application::commands::delete_project::DeleteProjectHandler;
use agent_application::commands::resume_agent::ResumeAgentHandler;
use agent_application::commands::set_office_additions::SetOfficeAdditionsHandler;
use agent_application::commands::set_setting::SetSettingHandler;
use agent_application::commands::set_task_text::SetTaskTextHandler;
use agent_application::commands::start_agent::StartAgentHandler;
use agent_application::commands::start_new_task::StartNewTaskHandler;
use agent_application::commands::stop_agent::StopAgentHandler;
use agent_application::common::action_journal::ActionJournal;
use agent_application::common::errors::{PortError, TerminalHostError};
use agent_application::common::interfaces::{
    ActionRecorder, AdditionsStore, AgentStore, CliAdapter, CliAdapters, Clock, DiffRequest,
    FailureReporter, FileView, FileViewError, ProjectStore, SessionStore, SettingsStore,
    TerminalHost,
};
use agent_application::common::models::{
    AgentActionEntry, ChangeList, CommitDetail, CommitPage, FileContent, FileDiff, FolderListing,
    PrepareLaunch, PreparedLaunch, StartTerminal,
};
use agent_application::common::session_ender::SessionEnder;
use agent_application::common::session_launcher::{LauncherServices, SessionLauncher};
use agent_application::queries::file_view::FileViewHandler;
use agent_application::queries::get_office_additions::GetOfficeAdditionsHandler;
use agent_application::queries::get_settings::GetSettingsHandler;
use agent_application::queries::list_agents::ListAgentsHandler;
use agent_application::queries::list_projects::ListProjectsHandler;
use agent_application::queries::list_sessions::ListSessionsHandler;
use agent_domain::entities::{AdditionsRecord, Agent};
use agent_domain::enums::AgentStatus;
use agent_domain::services::RelativePath;
use agent_domain::value_objects::{
    AgentCommand, DisplayName, ExitResult, MachineId, ProjectId, SessionId, Settings,
};
use kernel::{AgentId, Timestamp};
use serde_json::{Value, json};
use wire::{Request, Response};

use super::*;
use agent_application::commands::delete_agent::WorktreeOutcome;

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// These paths read and write only what their fakes hold.
fn never_called() -> ! {
    panic!("not used in these tests")
}

/// The stored agents, recording the order of the deletes.
#[derive(Default)]
struct FakeAgents {
    agents: Mutex<Vec<Agent>>,
    deleted: Mutex<Vec<AgentId>>,
}

impl FakeAgents {
    fn insert_new(&self, agent_id: AgentId, status: AgentStatus) {
        let agent = Agent::new(
            agent_id,
            DisplayName::parse("agent name", "Ana").expect("name"),
            AgentCommand::parse("claude").expect("command"),
            None,
            Some(ProjectId::generate()),
            MachineId::generate(),
            Timestamp::from_unix_millis(0),
        )
        .expect("agent");
        locked(&self.agents).push(Agent { status, ..agent });
    }

    fn delete_order(&self) -> Vec<AgentId> {
        locked(&self.deleted).clone()
    }

    fn status_of(&self, agent_id: AgentId) -> Option<AgentStatus> {
        locked(&self.agents)
            .iter()
            .find(|agent| agent.id == agent_id)
            .map(|agent| agent.status)
    }
}

impl AgentStore for FakeAgents {
    fn insert(&self, agent: &Agent) -> Result<(), PortError> {
        locked(&self.agents).push(agent.clone());
        Ok(())
    }

    fn find(&self, agent_id: AgentId) -> Result<Option<Agent>, PortError> {
        Ok(locked(&self.agents)
            .iter()
            .find(|agent| agent.id == agent_id)
            .cloned())
    }

    fn list(&self) -> Result<Vec<Agent>, PortError> {
        never_called()
    }

    fn update_status(&self, agent_id: AgentId, status: AgentStatus) -> Result<(), PortError> {
        let mut agents = locked(&self.agents);
        let stored = agents
            .iter_mut()
            .find(|agent| agent.id == agent_id)
            .ok_or_else(|| PortError::NotFound {
                what: "agent".to_owned(),
            })?;
        stored.status = status;
        Ok(())
    }

    fn update_task(
        &self,
        agent_id: AgentId,
        task: Option<&agent_domain::value_objects::TaskText>,
    ) -> Result<(), PortError> {
        let mut agents = locked(&self.agents);
        let stored = agents
            .iter_mut()
            .find(|agent| agent.id == agent_id)
            .ok_or_else(|| PortError::NotFound {
                what: "agent".to_owned(),
            })?;
        stored.task = task.cloned();
        Ok(())
    }

    fn delete(&self, agent_id: AgentId) -> Result<(), PortError> {
        locked(&self.agents).retain(|agent| agent.id != agent_id);
        locked(&self.deleted).push(agent_id);
        Ok(())
    }
}

#[derive(Default)]
struct FakeProjects;

impl ProjectStore for FakeProjects {
    fn insert(&self, _project: &agent_domain::entities::Project) -> Result<(), PortError> {
        never_called()
    }

    fn find(&self, _id: ProjectId) -> Result<Option<agent_domain::entities::Project>, PortError> {
        never_called()
    }

    fn list(&self) -> Result<Vec<agent_domain::entities::Project>, PortError> {
        never_called()
    }

    fn delete(&self, _id: ProjectId) -> Result<(), PortError> {
        never_called()
    }
}

#[derive(Default)]
struct FakeSettings;

impl SettingsStore for FakeSettings {
    fn load(&self) -> Result<Settings, PortError> {
        Ok(Settings::default())
    }

    fn save(&self, _settings: &Settings) -> Result<(), PortError> {
        never_called()
    }
}

#[derive(Default)]
struct FakeAdditions;

impl AdditionsStore for FakeAdditions {
    fn load(&self, _agent_id: AgentId) -> Result<Option<AdditionsRecord>, PortError> {
        never_called()
    }

    fn save(&self, _agent_id: AgentId, _record: &AdditionsRecord) -> Result<(), PortError> {
        never_called()
    }
}

/// The stored sessions; only an end of a running session reaches it.
#[derive(Default)]
struct FakeSessions;

impl SessionStore for FakeSessions {
    fn insert(&self, _session: &agent_domain::entities::Session) -> Result<(), PortError> {
        never_called()
    }

    fn find(&self, _id: SessionId) -> Result<Option<agent_domain::entities::Session>, PortError> {
        never_called()
    }

    fn list_for_agent(
        &self,
        _agent_id: AgentId,
    ) -> Result<Vec<agent_domain::entities::Session>, PortError> {
        never_called()
    }

    fn latest_for_agent(
        &self,
        _agent_id: AgentId,
    ) -> Result<Option<agent_domain::entities::Session>, PortError> {
        never_called()
    }

    fn set_cli_session_id(&self, _id: SessionId, _cli_session_id: &str) -> Result<(), PortError> {
        never_called()
    }

    fn end_running(
        &self,
        _agent_id: AgentId,
        _result: &ExitResult,
        _at: Timestamp,
    ) -> Result<bool, PortError> {
        Ok(true)
    }
}

#[derive(Default)]
struct FakeTerminals;

impl TerminalHost for FakeTerminals {
    fn prepare(&self, _request: &PrepareLaunch) -> Result<PreparedLaunch, TerminalHostError> {
        never_called()
    }

    fn validate_arguments(
        &self,
        _prepared: &PreparedLaunch,
        _arguments: &[String],
    ) -> Result<(), TerminalHostError> {
        never_called()
    }

    fn start(&self, _request: &StartTerminal) -> Result<(), TerminalHostError> {
        never_called()
    }

    fn stop(
        &self,
        _agent_id: AgentId,
        _grace: std::time::Duration,
    ) -> Result<(), TerminalHostError> {
        Ok(())
    }
}

#[derive(Default)]
struct FakeAdapters;

impl CliAdapters for FakeAdapters {
    fn for_program(&self, _program: &str) -> Arc<dyn CliAdapter> {
        never_called()
    }
}

#[derive(Default)]
struct FixedClock;

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_millis(1_700_000_000_000)
    }
}

#[derive(Default)]
struct RecordingRecorder;

impl ActionRecorder for RecordingRecorder {
    fn record(&self, _entry: &AgentActionEntry) -> Result<(), PortError> {
        Ok(())
    }
}

#[derive(Default)]
/// The fake adapters know no status command, so this never runs.
struct NoStatusCommands;

impl agent_application::common::interfaces::StatusCommandRunner for NoStatusCommands {
    fn run(
        &self,
        _run: &agent_application::common::models::StatusRun,
    ) -> agent_application::common::models::StatusRunResult {
        agent_application::common::models::StatusRunResult::CouldNotRun(
            "no status command in these tests".to_owned(),
        )
    }
}

struct FakeFailures;

impl FailureReporter for FakeFailures {
    fn report(&self, _error: &PortError) {
        never_called()
    }
}

/// The subtree answers the host bridge would give, so these tests exercise
/// the handler's own work: the routing, the events, and the failure mapping.
/// The bridge's stop order and confirmation rules have their own tests.
#[derive(Default)]
struct FakeSubtree {
    stopped: Mutex<Vec<AgentId>>,
    deleted: Mutex<Vec<AgentId>>,
    failure: Mutex<Option<SubtreeResult>>,
}

impl FakeSubtree {
    fn stopping(self, ids: &[AgentId]) -> Self {
        *locked(&self.stopped) = ids.to_vec();
        self
    }

    fn deleting(self, ids: &[AgentId]) -> Self {
        *locked(&self.deleted) = ids.to_vec();
        self
    }

    fn failing(self, result: SubtreeResult) -> Self {
        *locked(&self.failure) = Some(result);
        self
    }
}

impl SubtreeRequests for FakeSubtree {
    fn stop_subtree(&self, _request: &Request, _agent_id: AgentId) -> SubtreeResult {
        if let Some(failure) = &*locked(&self.failure) {
            return failure.clone();
        }
        SubtreeResult::Stopped(locked(&self.stopped).clone())
    }

    fn delete_subtree(
        &self,
        _request: &Request,
        _agent_id: AgentId,
        _worktrees: &DeleteWorktrees,
    ) -> SubtreeResult {
        if let Some(failure) = &*locked(&self.failure) {
            return failure.clone();
        }
        SubtreeResult::Deleted {
            agents: locked(&self.deleted).clone(),
            worktrees: WorktreeOutcome::default(),
        }
    }
}

/// The stores behind the use cases, cloned into every handler that shares
/// them.
struct FakeStores {
    agents: Arc<FakeAgents>,
    agents_trait: Arc<dyn AgentStore>,
    settings: Arc<dyn SettingsStore>,
    additions: Arc<dyn AdditionsStore>,
    sessions: Arc<dyn SessionStore>,
    projects: Arc<dyn ProjectStore>,
    terminals: Arc<dyn TerminalHost>,
    adapters: Arc<dyn CliAdapters>,
    clock: Arc<dyn Clock>,
}

fn stores() -> FakeStores {
    let agents = Arc::new(FakeAgents::default());
    FakeStores {
        agents_trait: Arc::clone(&agents) as Arc<dyn AgentStore>,
        agents,
        settings: Arc::new(FakeSettings),
        additions: Arc::new(FakeAdditions),
        sessions: Arc::new(FakeSessions),
        projects: Arc::new(FakeProjects),
        terminals: Arc::new(FakeTerminals),
        adapters: Arc::new(FakeAdapters),
        clock: Arc::new(FixedClock),
    }
}

fn launcher_of(stores: &FakeStores) -> SessionLauncher {
    SessionLauncher::new(
        Arc::clone(&stores.agents_trait),
        Arc::clone(&stores.projects),
        Arc::clone(&stores.additions),
        Arc::clone(&stores.sessions),
        LauncherServices {
            terminals: Arc::clone(&stores.terminals),
            adapters: Arc::clone(&stores.adapters),
            clock: Arc::clone(&stores.clock),
            failures: Arc::new(FakeFailures),
            status_commands: Arc::new(NoStatusCommands),
            sign_in_timeout: agent_application::common::sign_in_check::SIGN_IN_TIMEOUT,
        },
    )
}

fn use_cases(stores: &FakeStores) -> AgentUseCases {
    let journal = ActionJournal::new(Arc::new(RecordingRecorder), Arc::clone(&stores.clock));
    let ender = SessionEnder::new(
        Arc::clone(&stores.agents_trait),
        Arc::clone(&stores.settings),
        Arc::clone(&stores.sessions),
        Arc::clone(&stores.terminals),
        Arc::clone(&stores.clock),
    );
    let launcher = launcher_of(stores);
    AgentUseCases {
        create_agent: CreateAgentHandler::new(
            Arc::clone(&stores.agents_trait),
            Arc::clone(&stores.projects),
            Arc::clone(&stores.additions),
            Arc::clone(&stores.clock),
            journal.clone(),
            MachineId::generate(),
        ),
        delete_agent: DeleteAgentHandler::new(
            Arc::clone(&stores.agents_trait),
            Arc::clone(&stores.settings),
            Arc::clone(&stores.terminals),
            journal.clone(),
        ),
        start_agent: StartAgentHandler::new(
            Arc::clone(&stores.agents_trait),
            launcher.clone(),
            journal.clone(),
        ),
        stop_agent: StopAgentHandler::new(
            Arc::clone(&stores.agents_trait),
            ender.clone(),
            journal.clone(),
        ),
        start_new_task: StartNewTaskHandler::new(
            Arc::clone(&stores.agents_trait),
            ender.clone(),
            launcher.clone(),
            journal.clone(),
        ),
        continue_session: ContinueSessionHandler::new(
            Arc::clone(&stores.agents_trait),
            Arc::clone(&stores.sessions),
            Arc::clone(&stores.adapters),
            ender,
            launcher.clone(),
            journal.clone(),
        ),
        resume_agent: ResumeAgentHandler::new(
            Arc::clone(&stores.agents_trait),
            Arc::clone(&stores.sessions),
            Arc::clone(&stores.adapters),
            launcher,
            journal.clone(),
        ),
        set_office_additions: SetOfficeAdditionsHandler::new(
            Arc::clone(&stores.agents_trait),
            Arc::clone(&stores.additions),
            Arc::clone(&stores.clock),
        ),
        set_task_text: SetTaskTextHandler::new(
            Arc::clone(&stores.agents_trait),
            Arc::clone(&stores.clock),
            journal.clone(),
        ),
        get_office_additions: GetOfficeAdditionsHandler::new(
            Arc::clone(&stores.agents_trait),
            Arc::clone(&stores.additions),
        ),
        list_sessions: ListSessionsHandler::new(
            Arc::clone(&stores.agents_trait),
            Arc::clone(&stores.sessions),
            Arc::clone(&stores.adapters),
        ),
        list_agents: ListAgentsHandler::new(Arc::clone(&stores.agents_trait)),
        create_project: CreateProjectHandler::new(Arc::clone(&stores.projects)),
        delete_project: DeleteProjectHandler::new(Arc::clone(&stores.projects)),
        list_projects: ListProjectsHandler::new(Arc::clone(&stores.projects)),
        set_setting: SetSettingHandler::new(Arc::clone(&stores.settings)),
        get_settings: GetSettingsHandler::new(Arc::clone(&stores.settings)),
        worktree_info: WorktreeInfoHandler::new(Arc::clone(&stores.projects), None),
        worktree_status: WorktreeStatusHandler::new(Arc::clone(&stores.agents_trait), None),
        file_view: FileViewHandler::new(
            Arc::clone(&stores.agents_trait),
            Arc::clone(&stores.projects),
            Arc::new(FakeFileView),
        ),
    }
}

/// Lists an empty folder, refuses the path `escape`, and reads nothing else.
struct FakeFileView;

impl FileView for FakeFileView {
    fn list(
        &self,
        root: &std::path::Path,
        path: &RelativePath,
    ) -> Result<FolderListing, FileViewError> {
        if path.name() == Some("escape") {
            return Err(FileViewError::Refused("outside".to_owned()));
        }
        Ok(FolderListing {
            root: root.display().to_string(),
            path: path.to_text(),
            entries: Vec::new(),
            more: 0,
            read_at: Timestamp::from_unix_millis(5),
        })
    }

    fn read(
        &self,
        _: &std::path::Path,
        _: &RelativePath,
        _: bool,
    ) -> Result<FileContent, FileViewError> {
        never_called()
    }

    fn changes(&self, _: &std::path::Path) -> Result<ChangeList, FileViewError> {
        never_called()
    }

    fn diff(&self, _: &std::path::Path, _: &DiffRequest) -> Result<FileDiff, FileViewError> {
        never_called()
    }

    fn history(&self, _: &std::path::Path, _: u32) -> Result<CommitPage, FileViewError> {
        never_called()
    }

    fn commit(&self, _: &std::path::Path, _: &str) -> Result<CommitDetail, FileViewError> {
        Err(FileViewError::Failed("git is not available".to_owned()))
    }
}

/// The handlers with the subtree bridge attached, as the host wires them.
fn office(subtree: FakeSubtree) -> (Arc<FakeAgents>, AgentHandlers) {
    let stores = stores();
    let handlers = AgentHandlers::new(use_cases(&stores));
    handlers.with_subtree(Arc::new(subtree) as Arc<dyn SubtreeRequests>);
    (stores.agents, handlers)
}

/// The handlers without a bridge, the 0001 single-agent behavior.
fn single_agent_office() -> (Arc<FakeAgents>, AgentHandlers) {
    let stores = stores();
    let handlers = AgentHandlers::new(use_cases(&stores));
    (stores.agents, handlers)
}

fn request(kind: &str, fields: &Value) -> Request {
    Request {
        id: 1,
        kind: kind.to_owned(),
        fields: fields.as_object().cloned().unwrap_or_default(),
    }
}

fn failure_of(handled: &Handled) -> (&'static str, String) {
    let Response::Failure { code, message, .. } = &handled.response else {
        panic!("expected a failure, got {:?}", handled.response);
    };
    (code.as_str(), message.clone())
}

fn event_jsons(handled: &Handled) -> Vec<Value> {
    handled
        .events
        .iter()
        .map(|event| serde_json::from_str(&event.encode()).expect("event json"))
        .collect()
}

fn agent_id_field(agent_id: AgentId) -> Value {
    json!({ "agent_id": agent_id.as_uuid_text() })
}

#[test]
fn a_delete_with_the_bridge_emits_one_deleted_event_per_agent() {
    let lead = AgentId::generate();
    let member = AgentId::generate();
    let (_agents, handlers) = office(FakeSubtree::default().deleting(&[member, lead]));

    let handled = handlers.handle(&request("agent.delete", &agent_id_field(lead)));

    assert!(matches!(handled.response, Response::Success { .. }));
    let deleted = event_jsons(&handled);
    assert_eq!(deleted.len(), 2);
    assert_eq!(deleted[0]["type"], "agent.deleted");
    assert_eq!(deleted[0]["agent_id"], member.as_uuid_text());
    assert_eq!(deleted[1]["agent_id"], lead.as_uuid_text());
}

#[test]
fn a_stop_with_the_bridge_emits_one_stopped_event_per_agent() {
    let lead = AgentId::generate();
    let member = AgentId::generate();
    let (_agents, handlers) = office(FakeSubtree::default().stopping(&[member, lead]));

    let handled = handlers.handle(&request(
        "agent.stop",
        &json!({ "agent_id": lead.as_uuid_text(), "include_descendants": true }),
    ));

    assert!(matches!(handled.response, Response::Success { .. }));
    let statuses = event_jsons(&handled);
    assert_eq!(statuses.len(), 2);
    assert_eq!(statuses[0]["type"], "agent.status");
    assert_eq!(statuses[0]["agent_id"], member.as_uuid_text());
    assert_eq!(statuses[0]["status"], "stopped");
    assert_eq!(statuses[1]["agent_id"], lead.as_uuid_text());
}

#[test]
fn a_failed_subtree_action_answers_the_wire_failure_without_events() {
    let lead = AgentId::generate();
    let (_agents, handlers) = office(FakeSubtree::default().failing(SubtreeResult::Failed {
        code: wire::ErrorCode::Conflict,
        message: "the agent still has 1 descendant".to_owned(),
    }));

    let handled = handlers.handle(&request("agent.delete", &agent_id_field(lead)));

    let (code, message) = failure_of(&handled);
    assert_eq!(code, "conflict");
    assert_eq!(message, "the agent still has 1 descendant");
    assert_eq!(handled.events, []);
}

#[test]
fn a_delete_without_the_bridge_keeps_the_single_agent_behavior() {
    let (agents, handlers) = single_agent_office();
    let agent_id = AgentId::generate();
    agents.insert_new(agent_id, AgentStatus::Stopped);

    let handled = handlers.handle(&request("agent.delete", &agent_id_field(agent_id)));

    assert!(matches!(handled.response, Response::Success { .. }));
    assert_eq!(agents.delete_order(), [agent_id]);
    let deleted = event_jsons(&handled);
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0]["agent_id"], agent_id.as_uuid_text());
}

#[test]
fn a_stop_without_the_bridge_keeps_the_single_agent_behavior() {
    let (agents, handlers) = single_agent_office();
    let agent_id = AgentId::generate();
    agents.insert_new(agent_id, AgentStatus::Working);

    let handled = handlers.handle(&request("agent.stop", &agent_id_field(agent_id)));

    assert!(matches!(handled.response, Response::Success { .. }));
    assert_eq!(agents.status_of(agent_id), Some(AgentStatus::Stopped));
    let statuses = event_jsons(&handled);
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0]["status"], "stopped");
}

#[test]
fn a_second_bridge_attach_is_refused_and_the_first_stays_in_place() {
    let stores = stores();
    let handlers = AgentHandlers::new(use_cases(&stores));
    let deleted_agent = AgentId::generate();
    let first = Arc::new(FakeSubtree::default().deleting(&[deleted_agent]));
    let second = Arc::new(FakeSubtree::default());

    assert!(handlers.attach_subtree(Arc::clone(&first) as Arc<dyn SubtreeRequests>));
    assert!(!handlers.attach_subtree(Arc::clone(&second) as Arc<dyn SubtreeRequests>));

    // The refused attach changed nothing: the first bridge still answers.
    let handled = handlers.handle(&request("agent.delete", &agent_id_field(deleted_agent)));
    assert!(matches!(handled.response, Response::Success { .. }));
    assert_eq!(*locked(&first.deleted), [deleted_agent]);
    assert_eq!(*locked(&second.deleted), []);
}

#[test]
fn a_task_text_request_sets_the_task_and_publishes_one_task_event() {
    let agent_id = AgentId::generate();
    let (agents, handlers) = single_agent_office();
    agents.insert_new(agent_id, AgentStatus::Stopped);

    let handled = handlers.handle(&request(
        "agent.task.set",
        &json!({ "agent_id": agent_id.as_uuid_text(), "text": "Review PR 12" }),
    ));

    let Response::Success { result, .. } = &handled.response else {
        panic!("expected success, got {:?}", handled.response);
    };
    assert_eq!(result["task"]["text"], "Review PR 12");
    assert_eq!(result["task"]["source"], "user");
    let events = event_jsons(&handled);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["type"], "agent.task");
    assert_eq!(events[0]["agent_id"], agent_id.as_uuid_text());
    assert_eq!(events[0]["task"]["text"], "Review PR 12");
    assert_eq!(events[0].get("status"), None);
    assert_eq!(events[0].get("last_line"), None);
}

#[test]
fn a_task_text_past_the_limit_is_invalid_and_names_the_limit() {
    let agent_id = AgentId::generate();
    let (agents, handlers) = single_agent_office();
    agents.insert_new(agent_id, AgentStatus::Stopped);

    let handled = handlers.handle(&request(
        "agent.task.set",
        &json!({ "agent_id": agent_id.as_uuid_text(), "text": "x".repeat(8193) }),
    ));

    assert_eq!(
        failure_of(&handled),
        (
            "invalid",
            "a task text has at most 8,192 characters".to_owned()
        )
    );
    assert_eq!(handled.events.len(), 0);
}

#[test]
fn a_task_text_request_without_text_names_the_field() {
    let (_agents, handlers) = single_agent_office();

    let handled = handlers.handle(&request(
        "agent.task.set",
        &json!({ "agent_id": AgentId::generate().as_uuid_text() }),
    ));

    assert_eq!(
        failure_of(&handled),
        ("invalid", "text is required".to_owned())
    );
}

#[test]
fn a_resume_request_with_another_field_is_refused_by_name() {
    let (_agents, handlers) = single_agent_office();

    let handled = handlers.handle(&request(
        "agent.resume",
        &json!({ "agent_id": AgentId::generate().as_uuid_text(), "session_id": "x" }),
    ));

    assert_eq!(
        failure_of(&handled),
        (
            "invalid",
            "session_id is not a field of agent.resume".to_owned()
        )
    );
}

#[test]
fn a_resume_of_an_agent_that_is_not_interrupted_is_a_conflict_without_events() {
    let agent_id = AgentId::generate();
    let (agents, handlers) = single_agent_office();
    agents.insert_new(agent_id, AgentStatus::Stopped);

    let handled = handlers.handle(&request(
        "agent.resume",
        &json!({ "agent_id": agent_id.as_uuid_text() }),
    ));

    assert_eq!(
        failure_of(&handled),
        ("conflict", "the agent is no longer interrupted".to_owned())
    );
    assert_eq!(handled.events.len(), 0);
}

#[test]
fn a_resume_of_an_unknown_agent_is_not_found() {
    let (_agents, handlers) = single_agent_office();

    let handled = handlers.handle(&request(
        "agent.resume",
        &json!({ "agent_id": AgentId::generate().as_uuid_text() }),
    ));

    assert_eq!(failure_of(&handled).0, "not_found");
}

/// An absolute folder on every platform; nothing is created there.
fn work_folder() -> std::path::PathBuf {
    std::env::temp_dir().join("work")
}

fn agent_in_work_folder(agents: &FakeAgents) -> AgentId {
    let agent = Agent::new(
        AgentId::generate(),
        DisplayName::parse("agent name", "Ana").expect("name"),
        AgentCommand::parse("claude").expect("command"),
        Some(work_folder()),
        None,
        MachineId::generate(),
        Timestamp::from_unix_millis(0),
    )
    .expect("agent");
    let agent_id = agent.id;
    agents.insert(&agent).expect("insert");
    agent_id
}

#[test]
fn a_files_request_answers_with_the_contract_fields_and_publishes_nothing() {
    let (agents, handlers) = single_agent_office();
    let agent_id = agent_in_work_folder(&agents);

    let handled = handlers.handle(&request(
        "agent.files.list",
        &json!({"agent_id": agent_id.as_uuid_text(), "path": "src"}),
    ));

    let Response::Success { result, .. } = &handled.response else {
        panic!("expected success, got {:?}", handled.response);
    };
    assert_eq!(result["root"], work_folder().display().to_string());
    assert_eq!(result["path"], "src");
    assert_eq!(result["entries"], json!([]));
    assert_eq!(result["read_at"], 5);
    assert_eq!(handled.events.len(), 0);
}

#[test]
fn a_files_request_names_the_field_at_fault() {
    let (agents, handlers) = single_agent_office();
    let agent_id = agent_in_work_folder(&agents);
    let id = agent_id.as_uuid_text();

    let cases = [
        (
            "agent.files.read",
            json!({"agent_id": id}),
            "path is required",
        ),
        (
            "agent.files.read",
            json!({"agent_id": id, "path": "a", "reveal": "yes"}),
            "reveal must be true or false",
        ),
        (
            "agent.files.history",
            json!({"agent_id": id, "skip": -1}),
            "skip must be a whole number of at least 0",
        ),
        (
            "agent.files.commit",
            json!({"agent_id": id}),
            "commit is required",
        ),
        (
            "agent.files.list",
            json!({"agent_id": id, "path": "a/../b"}),
            "path must not have a . or .. part",
        ),
    ];

    for (kind, fields, message) in cases {
        let (code, text) = failure_of(&handlers.handle(&request(kind, &fields)));
        assert_eq!((code, text.as_str()), ("invalid", message), "{kind}");
    }
}

#[test]
fn a_files_request_maps_the_failures_to_the_contract_codes() {
    let (agents, handlers) = single_agent_office();
    let agent_id = agent_in_work_folder(&agents);
    let id = agent_id.as_uuid_text();

    let unknown = handlers.handle(&request(
        "agent.files.list",
        &json!({"agent_id": AgentId::generate().as_uuid_text(), "path": ""}),
    ));
    let refused = handlers.handle(&request(
        "agent.files.list",
        &json!({"agent_id": id, "path": "escape"}),
    ));
    let internal = handlers.handle(&request(
        "agent.files.commit",
        &json!({"agent_id": id, "commit": "a".repeat(40)}),
    ));

    assert_eq!(failure_of(&unknown).0, "not_found");
    assert_eq!(failure_of(&refused).0, "refused");
    assert_eq!(
        failure_of(&internal),
        ("internal", "git is not available".to_owned())
    );
}
