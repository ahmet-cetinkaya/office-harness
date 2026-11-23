// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_application::commands::change_agent_status::ChangeAgentStatusHandler;
use agent_application::commands::continue_session::ContinueSessionHandler;
use agent_application::commands::create_agent::CreateAgentHandler;
use agent_application::commands::create_project::CreateProjectHandler;
use agent_application::commands::delete_agent::DeleteAgentHandler;
use agent_application::commands::delete_project::DeleteProjectHandler;
use agent_application::commands::end_session::EndSessionHandler;
use agent_application::commands::interrupt_agents::InterruptAgentsHandler;
use agent_application::commands::resume_agent::ResumeAgentHandler;
use agent_application::commands::set_office_additions::SetOfficeAdditionsHandler;
use agent_application::commands::set_setting::SetSettingHandler;
use agent_application::commands::set_task_text::SetTaskTextHandler;
use agent_application::commands::sleep_agent::SleepAgentHandler;
use agent_application::commands::start_agent::StartAgentHandler;
use agent_application::commands::start_new_task::StartNewTaskHandler;
use agent_application::commands::stop_agent::StopAgentHandler;
use agent_application::common::action_journal::ActionJournal;
use agent_application::common::interfaces::{
    AdditionsStore, AgentStore, CliAdapters, Clock, FileView, FinishedTempStore, ProjectStore,
    SessionStore, SettingsStore, SleepGate, StatusCommandRunner, TerminalHost, WorktreeManager,
};
use agent_application::common::session_ender::SessionEnder;
use agent_application::common::session_launcher::{LauncherServices, SessionLauncher};
use agent_application::common::sign_in_check::SIGN_IN_TIMEOUT;
use agent_application::queries::file_view::FileViewHandler;
use agent_application::queries::get_office_additions::GetOfficeAdditionsHandler;
use agent_application::queries::get_settings::GetSettingsHandler;
use agent_application::queries::list_agents::ListAgentsHandler;
use agent_application::queries::list_projects::ListProjectsHandler;
use agent_application::queries::list_sessions::ListSessionsHandler;
use agent_application::queries::worktrees::{WorktreeInfoHandler, WorktreeStatusHandler};
use agent_domain::value_objects::MachineId;
use agent_infrastructure::persistence::{
    AdditionsRepository, AgentRepository, FinishedTempRepository, ProjectRepository,
    SessionRepository, SettingsRepository, SharedConnection,
};
use agent_infrastructure::services::{AttributionTiming, CliAdapterRegistry};
use agent_presentation::{AgentHandlers, AgentUseCases};
use audit_application::commands::record_action::RecordActionHandler;
use audit_application::common::interfaces::ActionLog;
use audit_application::queries::list_actions::ListActionsHandler;
use audit_application::queries::list_office_actions::ListOfficeActionsHandler;
use audit_infrastructure::persistence::ActionRepository;
use audit_presentation::AuditHandlers;

use super::{ServerLog, StorageAlerts, TerminalServices};
use crate::bridges::{
    ActionRecorderBridge, AgentCalls, IdleSleepGate, SavedOutputs, SignInWatch,
    StatusCommandBridge, StatusOrder, StoppingFlag, SystemClock, TaskOrder, TerminalHostBridge,
};

/// The agent and audit modules, wired: the use cases behind their handlers,
/// and the handler that terminal events use to change a status.
pub struct AgentServices {
    pub handlers: Arc<AgentHandlers>,
    pub audit: AuditHandlers,
    pub change_status: ChangeAgentStatusHandler,
    /// Closes a session when its terminal reports an exit.
    pub end_session: EndSessionHandler,
    /// Ends the running agents when the server stops.
    pub interrupt_agents: InterruptAgentsHandler,
    /// Names the output file that `terminal.attach` shows for an agent that
    /// does not run.
    pub saved_outputs: SavedOutputs,
    /// Orders `agent.start` before the statuses its terminal causes.
    pub status_order: StatusOrder,
    /// Orders the storing and the publishing of every task text change.
    pub task_order: TaskOrder,
    /// Set by the interrupt at stop, so no start follows it.
    pub stopping: StoppingFlag,
    /// The action record the agent use cases write. The hierarchy's
    /// directive journal shares it, so a directive is kept like any other
    /// action (FR-011).
    pub journal: ActionJournal,
    /// Writes the hold, release, and automatic-hold entries (0007 FR-010).
    pub record_action: RecordActionHandler,
    /// The office-level entries, for `office.actions`.
    pub office_actions: ListOfficeActionsHandler,
    /// The entries of one agent; the temp watcher reads who stopped a temp.
    pub actions: ListActionsHandler,
    /// Reads back an agent that another module's request created (the lead
    /// of `team.create`), so its `agent.changed` event can be sent.
    pub agent_reader: ListAgentsHandler,
    /// Decides which idle agents may sleep (0018). The services it reads are
    /// built after this module, so startup connects them later.
    pub sleep_gate: Arc<IdleSleepGate>,
    /// The settings as the use cases read them; the idle sleep task reads
    /// the idle time here on every round.
    pub settings: Arc<dyn SettingsStore>,
    /// Runs a start's sign-in check before the status order (0019 D13).
    pub sign_in_checks: SessionLauncher,
    /// The sessions as the temp rules read them: the agent of a session, and
    /// the exit result of a finished run (0020).
    pub sessions: Arc<dyn SessionStore>,
    /// The entries of the temps that ended (0020).
    pub finished_temps: Arc<dyn FinishedTempStore>,
}

/// The stores and services that the use cases share.
struct Wiring {
    agents: Arc<dyn AgentStore>,
    projects: Arc<dyn ProjectStore>,
    settings: Arc<dyn SettingsStore>,
    additions: Arc<dyn AdditionsStore>,
    sessions: Arc<dyn SessionStore>,
    adapters: Arc<dyn CliAdapters>,
    clock: Arc<dyn Clock>,
    terminals: Arc<dyn TerminalHost>,
    status_commands: Arc<dyn StatusCommandRunner>,
    journal: ActionJournal,
    alerts: StorageAlerts,
    worktrees: Arc<dyn WorktreeManager>,
    file_view: Arc<dyn FileView>,
    sleep_gate: Arc<dyn SleepGate>,
}

/// Joins the agent use cases to the SQLite stores, the terminal, the clock,
/// and the action record. Every store uses the one `connection`.
#[must_use]
pub fn build_agent_services(
    connection: &SharedConnection,
    machine_id: MachineId,
    terminal: &TerminalServices,
    alerts: &StorageAlerts,
    attribution: AttributionTiming,
    log: &ServerLog,
    agent_calls: AgentCalls,
) -> AgentServices {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let action_log: Arc<dyn ActionLog> = Arc::new(ActionRepository::new(connection.handle()));
    let record_action = RecordActionHandler::new(Arc::clone(&action_log));
    let recorder = Arc::new(ActionRecorderBridge::new(
        record_action.clone(),
        alerts.clone(),
    ));
    let sleep_gate = Arc::new(IdleSleepGate::default());
    let wiring = Wiring {
        agents: Arc::new(AgentRepository::new(connection.clone())),
        projects: Arc::new(ProjectRepository::new(connection.clone())),
        settings: Arc::new(SettingsRepository::new(connection.clone())),
        additions: Arc::new(AdditionsRepository::new(connection.clone())),
        sessions: Arc::new(SessionRepository::new(connection.clone())),
        adapters: Arc::new(CliAdapterRegistry::new(attribution)),
        terminals: Arc::new(
            TerminalHostBridge::new(
                terminal.launcher.clone(),
                Arc::clone(&terminal.runtime),
                terminal.events.clone(),
                log.clone(),
                agent_calls,
            )
            .with_person_input(terminal.person_input.clone())
            .with_sign_in_watch(SignInWatch {
                runtime: Arc::clone(&terminal.runtime),
                person_input: terminal.person_input.clone(),
                ends: terminal.sign_in_ends.clone(),
                timing: terminal.sign_in_timing,
                log: log.clone(),
            }),
        ),
        status_commands: Arc::new(StatusCommandBridge::new(terminal.launcher.clone())),
        journal: ActionJournal::new(recorder, Arc::clone(&clock)),
        alerts: alerts.clone(),
        clock,
        worktrees: Arc::clone(&terminal.worktrees),
        file_view: Arc::clone(&terminal.file_view),
        sleep_gate: Arc::clone(&sleep_gate) as Arc<dyn SleepGate>,
    };
    AgentServices {
        handlers: Arc::new(AgentHandlers::new(use_cases(&wiring, machine_id))),
        audit: AuditHandlers::new(ListActionsHandler::new(Arc::clone(&action_log))),
        actions: ListActionsHandler::new(Arc::clone(&action_log)),
        office_actions: ListOfficeActionsHandler::new(action_log),
        record_action,
        agent_reader: ListAgentsHandler::new(Arc::clone(&wiring.agents)),
        sign_in_checks: launcher_of(&wiring),
        change_status: ChangeAgentStatusHandler::new(Arc::clone(&wiring.agents)),
        end_session: EndSessionHandler::new(
            Arc::clone(&wiring.sessions),
            Arc::clone(&wiring.clock),
        ),
        interrupt_agents: InterruptAgentsHandler::new(
            Arc::clone(&wiring.agents),
            Arc::clone(&wiring.settings),
            Arc::clone(&wiring.sessions),
            Arc::clone(&wiring.terminals),
            Arc::clone(&wiring.clock),
        ),
        sessions: Arc::clone(&wiring.sessions),
        finished_temps: Arc::new(FinishedTempRepository::new(connection.clone())),
        saved_outputs: SavedOutputs::new(wiring.sessions),
        status_order: StatusOrder::default(),
        task_order: TaskOrder::default(),
        stopping: StoppingFlag::default(),
        journal: wiring.journal.clone(),
        sleep_gate,
        settings: Arc::clone(&wiring.settings),
    }
}

fn launcher_of(wiring: &Wiring) -> SessionLauncher {
    SessionLauncher::new(
        Arc::clone(&wiring.agents),
        Arc::clone(&wiring.projects),
        Arc::clone(&wiring.additions),
        Arc::clone(&wiring.sessions),
        LauncherServices {
            terminals: Arc::clone(&wiring.terminals),
            adapters: Arc::clone(&wiring.adapters),
            clock: Arc::clone(&wiring.clock),
            failures: Arc::new(wiring.alerts.clone()),
            status_commands: Arc::clone(&wiring.status_commands),
            sign_in_timeout: SIGN_IN_TIMEOUT,
        },
    )
}

fn sleep_handler(wiring: &Wiring, ender: SessionEnder) -> SleepAgentHandler {
    SleepAgentHandler::new(
        Arc::clone(&wiring.agents),
        ender,
        wiring.journal.clone(),
        Arc::clone(&wiring.sleep_gate),
    )
}

fn use_cases(wiring: &Wiring, machine_id: MachineId) -> AgentUseCases {
    let ender = SessionEnder::new(
        Arc::clone(&wiring.agents),
        Arc::clone(&wiring.settings),
        Arc::clone(&wiring.sessions),
        Arc::clone(&wiring.terminals),
        Arc::clone(&wiring.clock),
    );
    let launcher = launcher_of(wiring);
    let journal = &wiring.journal;
    AgentUseCases {
        create_agent: CreateAgentHandler::new(
            Arc::clone(&wiring.agents),
            Arc::clone(&wiring.projects),
            Arc::clone(&wiring.additions),
            Arc::clone(&wiring.clock),
            Arc::clone(&wiring.settings),
            journal.clone(),
            machine_id,
        )
        .with_worktrees(Arc::clone(&wiring.worktrees)),
        delete_agent: DeleteAgentHandler::new(
            Arc::clone(&wiring.agents),
            Arc::clone(&wiring.settings),
            Arc::clone(&wiring.terminals),
            journal.clone(),
        )
        .with_worktrees(Arc::clone(&wiring.worktrees)),
        start_agent: StartAgentHandler::new(
            Arc::clone(&wiring.agents),
            launcher.clone(),
            journal.clone(),
        ),
        stop_agent: StopAgentHandler::new(
            Arc::clone(&wiring.agents),
            ender.clone(),
            journal.clone(),
        ),
        start_new_task: StartNewTaskHandler::new(
            Arc::clone(&wiring.agents),
            ender.clone(),
            launcher.clone(),
            journal.clone(),
        ),
        continue_session: ContinueSessionHandler::new(
            Arc::clone(&wiring.agents),
            Arc::clone(&wiring.sessions),
            Arc::clone(&wiring.adapters),
            ender.clone(),
            launcher.clone(),
            journal.clone(),
        ),
        resume_agent: ResumeAgentHandler::new(
            Arc::clone(&wiring.agents),
            Arc::clone(&wiring.sessions),
            Arc::clone(&wiring.adapters),
            launcher,
            journal.clone(),
        ),
        sleep_agent: sleep_handler(wiring, ender),
        set_office_additions: SetOfficeAdditionsHandler::new(
            Arc::clone(&wiring.agents),
            Arc::clone(&wiring.additions),
            Arc::clone(&wiring.clock),
        ),
        set_task_text: SetTaskTextHandler::new(
            Arc::clone(&wiring.agents),
            Arc::clone(&wiring.clock),
            journal.clone(),
        ),
        get_office_additions: GetOfficeAdditionsHandler::new(
            Arc::clone(&wiring.agents),
            Arc::clone(&wiring.additions),
        ),
        list_sessions: ListSessionsHandler::new(
            Arc::clone(&wiring.agents),
            Arc::clone(&wiring.sessions),
            Arc::clone(&wiring.adapters),
        ),
        list_agents: ListAgentsHandler::new(Arc::clone(&wiring.agents)),
        create_project: CreateProjectHandler::new(Arc::clone(&wiring.projects)),
        delete_project: DeleteProjectHandler::new(Arc::clone(&wiring.projects)),
        list_projects: ListProjectsHandler::new(Arc::clone(&wiring.projects)),
        set_setting: SetSettingHandler::new(Arc::clone(&wiring.settings)),
        get_settings: GetSettingsHandler::new(Arc::clone(&wiring.settings)),
        worktree_info: WorktreeInfoHandler::new(
            Arc::clone(&wiring.projects),
            Some(Arc::clone(&wiring.worktrees)),
        ),
        worktree_status: WorktreeStatusHandler::new(
            Arc::clone(&wiring.agents),
            Some(Arc::clone(&wiring.worktrees)),
        ),
        file_view: FileViewHandler::new(
            Arc::clone(&wiring.agents),
            Arc::clone(&wiring.projects),
            Arc::clone(&wiring.file_view),
        ),
    }
}
