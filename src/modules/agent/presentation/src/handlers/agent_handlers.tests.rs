// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex};

use agent_application::commands::continue_session::ContinueSessionHandler;
use agent_application::commands::create_agent::CreateAgentHandler;
use agent_application::commands::create_project::CreateProjectHandler;
use agent_application::commands::delete_agent::DeleteAgentHandler;
use agent_application::commands::delete_project::DeleteProjectHandler;
use agent_application::commands::set_office_additions::SetOfficeAdditionsHandler;
use agent_application::commands::set_setting::SetSettingHandler;
use agent_application::commands::start_agent::StartAgentHandler;
use agent_application::commands::start_new_task::StartNewTaskHandler;
use agent_application::commands::stop_agent::StopAgentHandler;
use agent_application::common::action_journal::ActionJournal;
use agent_application::common::errors::{PortError, TerminalHostError};
use agent_application::common::interfaces::{
    ActionRecorder, AdditionsStore, AgentStore, CliAdapter, CliAdapters, Clock, FailureReporter,
    ProjectStore, SessionStore, SettingsStore, TerminalHost,
};
use agent_application::common::models::{
    AgentActionEntry, PrepareLaunch, PreparedLaunch, StartTerminal,
};
use agent_application::common::session_ender::SessionEnder;
use agent_application::common::session_launcher::{LauncherServices, SessionLauncher};
use agent_application::queries::get_office_additions::GetOfficeAdditionsHandler;
use agent_application::queries::get_settings::GetSettingsHandler;
use agent_application::queries::list_agents::ListAgentsHandler;
use agent_application::queries::list_projects::ListProjectsHandler;
use agent_application::queries::list_sessions::ListSessionsHandler;
use agent_domain::entities::{AdditionsRecord, Agent};
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{
    AgentCommand, DisplayName, ExitResult, MachineId, ProjectId, SessionId, Settings,
};
use hierarchy_application::commands::subtree::SubtreeHandler;
use hierarchy_application::common::interfaces::{HierarchyStore, TreeSnapshot};
use hierarchy_application::common::ports::{
    SubtreeActionError, SubtreeAgentEnder, SubtreeAgentRemover,
};
use hierarchy_application::{
    CreateTeamHandler, DeleteTeamHandler, HierarchyApplicationError, PlaceAgentHandler, TreeQuery,
};
use hierarchy_domain::{Role, Team, TeamId, TeamName, TreeRow};
use kernel::{AgentId, Timestamp};
use serde_json::{Value, json};
use wire::{Request, Response};

use super::*;

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
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

    fn count(&self) -> usize {
        locked(&self.agents).len()
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
struct FakeFailures;

impl FailureReporter for FakeFailures {
    fn report(&self, _error: &PortError) {
        never_called()
    }
}

/// The hierarchy rows the subtree use cases read back.
struct FixedRows(Vec<TreeRow>);

impl HierarchyStore for FixedRows {
    fn snapshot(&self) -> Result<TreeSnapshot, HierarchyApplicationError> {
        Ok(TreeSnapshot {
            rows: self.0.clone(),
            teams: Vec::new(),
        })
    }

    fn team_name_taken(&self, _key: &str) -> Result<bool, HierarchyApplicationError> {
        never_called()
    }

    fn save_team_and_place_lead(
        &self,
        _team: &Team,
        _parent_agent_id: Option<AgentId>,
    ) -> Result<(), HierarchyApplicationError> {
        never_called()
    }

    fn delete_team_and_unplace(&self, _team_id: TeamId) -> Result<(), HierarchyApplicationError> {
        never_called()
    }

    fn place_agent(
        &self,
        _agent_id: AgentId,
        _role: Option<Role>,
        _parent_agent_id: Option<AgentId>,
        _team_id: Option<TeamId>,
    ) -> Result<(), HierarchyApplicationError> {
        never_called()
    }

    fn team_name_of(
        &self,
        _team_id: TeamId,
    ) -> Result<Option<TeamName>, HierarchyApplicationError> {
        never_called()
    }
}

/// Bridges one agent stop to the fake-backed handler, as the host does.
struct StopAgentPort {
    stop: StopAgentHandler,
}

impl SubtreeAgentEnder for StopAgentPort {
    fn stop(&self, agent_id: AgentId) -> Result<(), SubtreeActionError> {
        self.stop
            .handle(agent_id)
            .map(|_| ())
            .map_err(|error| SubtreeActionError(error.to_string()))
    }
}

/// Bridges one agent delete to the fake-backed handler, as the host does.
struct DeleteAgentPort {
    delete: DeleteAgentHandler,
}

impl SubtreeAgentRemover for DeleteAgentPort {
    fn delete(&self, agent_id: AgentId) -> Result<(), SubtreeActionError> {
        self.delete
            .handle(agent_id)
            .map_err(|error| SubtreeActionError(error.to_string()))
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

fn use_cases(stores: &FakeStores) -> AgentUseCases {
    let journal = ActionJournal::new(Arc::new(RecordingRecorder), Arc::clone(&stores.clock));
    let ender = SessionEnder::new(
        Arc::clone(&stores.agents_trait),
        Arc::clone(&stores.settings),
        Arc::clone(&stores.sessions),
        Arc::clone(&stores.terminals),
        Arc::clone(&stores.clock),
    );
    let launcher = SessionLauncher::new(
        Arc::clone(&stores.agents_trait),
        Arc::clone(&stores.projects),
        Arc::clone(&stores.additions),
        Arc::clone(&stores.sessions),
        LauncherServices {
            terminals: Arc::clone(&stores.terminals),
            adapters: Arc::clone(&stores.adapters),
            clock: Arc::clone(&stores.clock),
            failures: Arc::new(FakeFailures),
        },
    );
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
            launcher,
            journal.clone(),
        ),
        set_office_additions: SetOfficeAdditionsHandler::new(
            Arc::clone(&stores.agents_trait),
            Arc::clone(&stores.additions),
            Arc::clone(&stores.clock),
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
    }
}

/// The handlers over the fakes, with the subtree use cases attached as the
/// host wires them. Returns the fake agents for the assertions.
fn office(rows: &[TreeRow]) -> (Arc<FakeAgents>, AgentHandlers) {
    let stores = stores();
    let use_cases = use_cases(&stores);
    let hierarchy_store: Arc<dyn HierarchyStore> = Arc::new(FixedRows(rows.to_vec()));
    let subtree = SubtreeHandler::new(
        Arc::clone(&hierarchy_store),
        Arc::new(StopAgentPort {
            stop: use_cases.stop_agent.clone(),
        }),
        Arc::new(DeleteAgentPort {
            delete: use_cases.delete_agent.clone(),
        }),
    );
    let handlers =
        AgentHandlers::new(use_cases).with_subtree(hierarchy_presentation::HierarchyUseCases {
            tree: Arc::new(TreeQuery::new(Arc::clone(&hierarchy_store))),
            create_team: CreateTeamHandler::new(Arc::clone(&hierarchy_store)),
            delete_team: DeleteTeamHandler::new(Arc::clone(&hierarchy_store)),
            place_agent: PlaceAgentHandler::new(Arc::clone(&hierarchy_store)),
            subtree,
        });
    (stores.agents, handlers)
}

/// lead > member, with both agents in the store under the given statuses.
fn lead_with_member(
    lead_status: AgentStatus,
    member_status: AgentStatus,
) -> (Arc<FakeAgents>, AgentHandlers, AgentId, AgentId) {
    let lead = AgentId::generate();
    let member = AgentId::generate();
    let rows = vec![
        TreeRow {
            agent_id: lead,
            role: Some(Role::Lead),
            parent_agent_id: None,
            team_id: None,
        },
        TreeRow {
            agent_id: member,
            role: Some(Role::Member),
            parent_agent_id: Some(lead),
            team_id: None,
        },
    ];
    let (agents, handlers) = office(&rows);
    agents.insert_new(lead, lead_status);
    agents.insert_new(member, member_status);
    (agents, handlers, lead, member)
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
fn a_delete_without_include_descendants_conflicts_and_deletes_nothing() {
    let (agents, handlers, lead, _member) =
        lead_with_member(AgentStatus::Stopped, AgentStatus::Stopped);

    let handled = handlers.handle(&request("agent.delete", &agent_id_field(lead)));

    let (code, message) = failure_of(&handled);
    assert_eq!(code, "conflict");
    assert!(message.contains("still has 1 descendant"), "{message}");
    assert_eq!(agents.count(), 2);
    assert_eq!(agents.delete_order(), []);
    assert_eq!(handled.events, []);
}

#[test]
fn a_delete_with_descendants_but_no_count_conflicts_and_deletes_nothing() {
    let (agents, handlers, lead, _member) =
        lead_with_member(AgentStatus::Stopped, AgentStatus::Stopped);
    let mut fields = agent_id_field(lead);
    fields["include_descendants"] = json!(true);

    let handled = handlers.handle(&request("agent.delete", &fields));

    let (code, message) = failure_of(&handled);
    assert_eq!(code, "conflict");
    assert!(message.contains("send descendant_count 1"), "{message}");
    assert_eq!(agents.count(), 2);
    assert_eq!(agents.delete_order(), []);
}

#[test]
fn a_delete_with_the_confirmed_count_deletes_the_member_then_the_lead() {
    let (agents, handlers, lead, member) =
        lead_with_member(AgentStatus::Stopped, AgentStatus::Stopped);
    let mut fields = agent_id_field(lead);
    fields["include_descendants"] = json!(true);
    fields["descendant_count"] = json!(1);

    let handled = handlers.handle(&request("agent.delete", &fields));

    assert!(matches!(handled.response, Response::Success { .. }));
    assert_eq!(agents.delete_order(), [member, lead]);
    assert_eq!(agents.count(), 0);
    let deleted = event_jsons(&handled);
    assert_eq!(deleted.len(), 2);
    assert_eq!(deleted[0]["type"], "agent.deleted");
    assert_eq!(deleted[0]["agent_id"], member.as_uuid_text());
    assert_eq!(deleted[1]["agent_id"], lead.as_uuid_text());
}

#[test]
fn a_delete_with_a_stale_count_conflicts_and_deletes_nothing() {
    let (agents, handlers, lead, _member) =
        lead_with_member(AgentStatus::Stopped, AgentStatus::Stopped);
    let mut fields = agent_id_field(lead);
    fields["include_descendants"] = json!(true);
    fields["descendant_count"] = json!(2);

    let handled = handlers.handle(&request("agent.delete", &fields));

    let (code, message) = failure_of(&handled);
    assert_eq!(code, "conflict");
    assert!(message.contains("has 1 descendant now"), "{message}");
    assert_eq!(agents.count(), 2);
    assert_eq!(agents.delete_order(), []);
}

#[test]
fn a_stop_of_a_subtree_is_unchanged_and_runs_member_first() {
    let (agents, handlers, lead, member) =
        lead_with_member(AgentStatus::Working, AgentStatus::Working);
    let mut fields = agent_id_field(lead);
    fields["include_descendants"] = json!(true);

    let handled = handlers.handle(&request("agent.stop", &fields));

    assert!(matches!(handled.response, Response::Success { .. }));
    assert_eq!(agents.status_of(member), Some(AgentStatus::Stopped));
    assert_eq!(agents.status_of(lead), Some(AgentStatus::Stopped));
    let statuses = event_jsons(&handled);
    assert_eq!(statuses.len(), 2);
    assert_eq!(statuses[0]["type"], "agent.status");
    assert_eq!(statuses[0]["agent_id"], member.as_uuid_text());
    assert_eq!(statuses[0]["status"], "stopped");
    assert_eq!(statuses[1]["agent_id"], lead.as_uuid_text());
}

#[test]
fn a_leaf_delete_still_needs_no_confirmation() {
    let (agents, handlers, lead, member) =
        lead_with_member(AgentStatus::Stopped, AgentStatus::Stopped);

    let handled = handlers.handle(&request("agent.delete", &agent_id_field(member)));

    assert!(matches!(handled.response, Response::Success { .. }));
    assert_eq!(agents.delete_order(), [member]);
    assert_eq!(agents.count(), 1);
    assert_eq!(agents.status_of(lead), Some(AgentStatus::Stopped));
}
