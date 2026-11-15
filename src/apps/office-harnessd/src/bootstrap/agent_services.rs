// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_application::commands::change_agent_status::ChangeAgentStatusHandler;
use agent_application::commands::create_agent::CreateAgentHandler;
use agent_application::commands::create_project::CreateProjectHandler;
use agent_application::commands::delete_agent::DeleteAgentHandler;
use agent_application::commands::delete_project::DeleteProjectHandler;
use agent_application::commands::set_setting::SetSettingHandler;
use agent_application::commands::start_agent::StartAgentHandler;
use agent_application::commands::stop_agent::StopAgentHandler;
use agent_application::common::action_journal::ActionJournal;
use agent_application::common::interfaces::{
    AgentStore, Clock, ProjectStore, SettingsStore, TerminalHost,
};
use agent_application::queries::get_settings::GetSettingsHandler;
use agent_application::queries::list_agents::ListAgentsHandler;
use agent_application::queries::list_projects::ListProjectsHandler;
use agent_domain::value_objects::MachineId;
use agent_infrastructure::persistence::{
    AgentRepository, ProjectRepository, SettingsRepository, SharedConnection,
};
use agent_presentation::{AgentHandlers, AgentUseCases};
use audit_application::commands::record_action::RecordActionHandler;
use audit_application::common::interfaces::ActionLog;
use audit_application::queries::list_actions::ListActionsHandler;
use audit_infrastructure::persistence::ActionRepository;
use audit_presentation::AuditHandlers;

use super::{StorageAlerts, TerminalServices};
use crate::bridges::{ActionRecorderBridge, StatusOrder, SystemClock, TerminalHostBridge};

/// The agent and audit modules, wired: the use cases behind their handlers,
/// and the handler that terminal events use to change a status.
pub struct AgentServices {
    pub handlers: Arc<AgentHandlers>,
    pub audit: AuditHandlers,
    pub change_status: ChangeAgentStatusHandler,
    /// Orders `agent.start` before the statuses its terminal causes.
    pub status_order: StatusOrder,
}

/// Joins the agent use cases to the SQLite stores, the terminal, the clock,
/// and the action record. Every store uses the one `connection`.
#[must_use]
pub fn build_agent_services(
    connection: &SharedConnection,
    machine_id: MachineId,
    terminal: &TerminalServices,
    alerts: &StorageAlerts,
) -> AgentServices {
    let agents: Arc<dyn AgentStore> = Arc::new(AgentRepository::new(connection.clone()));
    let projects: Arc<dyn ProjectStore> = Arc::new(ProjectRepository::new(connection.clone()));
    let settings: Arc<dyn SettingsStore> = Arc::new(SettingsRepository::new(connection.clone()));
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let terminals: Arc<dyn TerminalHost> = Arc::new(TerminalHostBridge::new(
        terminal.launcher.clone(),
        Arc::clone(&terminal.runtime),
    ));
    let action_log: Arc<dyn ActionLog> = Arc::new(ActionRepository::new(connection.handle()));
    let recorder = Arc::new(ActionRecorderBridge::new(
        RecordActionHandler::new(Arc::clone(&action_log)),
        alerts.clone(),
    ));
    let journal = ActionJournal::new(recorder, Arc::clone(&clock));

    let use_cases = AgentUseCases {
        create_agent: CreateAgentHandler::new(
            Arc::clone(&agents),
            Arc::clone(&projects),
            clock,
            journal.clone(),
            machine_id,
        ),
        delete_agent: DeleteAgentHandler::new(
            Arc::clone(&agents),
            Arc::clone(&settings),
            Arc::clone(&terminals),
            journal.clone(),
        ),
        start_agent: StartAgentHandler::new(
            Arc::clone(&agents),
            Arc::clone(&projects),
            Arc::clone(&terminals),
            journal.clone(),
        ),
        stop_agent: StopAgentHandler::new(
            Arc::clone(&agents),
            Arc::clone(&settings),
            terminals,
            journal,
        ),
        list_agents: ListAgentsHandler::new(Arc::clone(&agents)),
        create_project: CreateProjectHandler::new(Arc::clone(&projects)),
        delete_project: DeleteProjectHandler::new(Arc::clone(&projects)),
        list_projects: ListProjectsHandler::new(projects),
        set_setting: SetSettingHandler::new(Arc::clone(&settings)),
        get_settings: GetSettingsHandler::new(settings),
    };
    AgentServices {
        handlers: Arc::new(AgentHandlers::new(use_cases)),
        audit: AuditHandlers::new(ListActionsHandler::new(action_log)),
        change_status: ChangeAgentStatusHandler::new(agents),
        status_order: StatusOrder::default(),
    }
}
