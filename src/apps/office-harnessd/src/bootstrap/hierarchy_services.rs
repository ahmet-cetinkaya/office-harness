// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use crate::bridges::{delete_worktrees, worktree_report};
use agent_application::commands::delete_agent::DeleteAgentHandler;
use agent_application::commands::stop_agent::StopAgentHandler;
use agent_application::common::interfaces::AgentStore;
use agent_infrastructure::persistence::{AgentRepository, SharedConnection};
use hierarchy_application::commands::create_role_agent::CreateRoleAgentHandler;
use hierarchy_application::commands::hold::HoldService;
use hierarchy_application::commands::send_directive::{DirectivePorts, SendDirectiveHandler};
use hierarchy_application::commands::send_report::{ReportPorts, SendReportHandler};
use hierarchy_application::commands::subtree::SubtreeHandler;
use hierarchy_application::common::interfaces::{HierarchyStore, HoldStore};
use hierarchy_application::common::ports::{
    AgentCreator, DirectiveJournal, DirectiveTarget, HoldClock, HoldJournal, HoldNotifier,
    RateLimitSource, ReportJournal, SubtreeActionError, SubtreeAgentEnder, SubtreeAgentRemover,
    SubtreeWorktrees, WorktreeReport,
};
use hierarchy_application::{CreateTeamHandler, DeleteTeamHandler, PlaceAgentHandler, TreeQuery};
use hierarchy_infrastructure::persistence::migrations::{LEDGER_TABLE, SCRIPTS};
use hierarchy_infrastructure::persistence::repositories::hierarchy_repository::HierarchyRepository;
use hierarchy_infrastructure::persistence::repositories::hold_repository::HoldRepository;
use hierarchy_infrastructure::services::agent_places::AgentPlaces;
use hierarchy_presentation::{HierarchyHandlers, HierarchyUseCases};
use kernel::AgentId;

use super::directive_delivery::spawn_directive_delivery;
use super::{
    AgentServices, ServerLog, StartupError, StorageAlerts, TerminalServices, run_migrations,
};
use crate::bridges::{
    AgentCreatorBridge, DirectiveJournalBridge, DirectiveReceiptBridge, DirectiveTargetBridge,
    DirectiveWriterBridge, HoldJournalBridge, HoldNotifierBridge, NeedsYouBridge,
    NotificationBridge, PersonReportsBridge, RoleRequestsBridge, SettingsRateLimits, SteadyClock,
    SubtreeRequestsBridge,
};
use crate::server::EventHub;

/// The hierarchy services: the store, the use cases, and the handlers.
pub struct HierarchyServices {
    pub handlers: Arc<HierarchyHandlers>,
    /// The tree the agent-call gateway reads for the descendant rule
    /// (FR-008), the same store the use cases answer.
    pub tree: Arc<TreeQuery>,
    /// Sends one directive from a running agent to its member (FR-009). The
    /// agent-call server answers `directive.send` with it.
    pub send_directive: Arc<SendDirectiveHandler>,
    /// Sends one report from a running agent to its parent or to the person
    /// (0015). The agent-call server answers `report.send` with it.
    pub send_report: Arc<SendReportHandler>,
    /// Journals the directive attempts and the gateway's own refusals into
    /// the shared action record (FR-011).
    pub directive_journal: Arc<DirectiveJournalBridge>,
    /// The holds and the rate limits, one service for the agent-call gateway
    /// and the user's `hold.*` requests (0007).
    pub holds: Arc<HoldService>,
    /// The hierarchy store and the agent creator, which the template create
    /// bridge joins under the same tree exclusion (0009 D5).
    pub store: Arc<dyn HierarchyStore>,
    pub creator: Arc<dyn AgentCreator>,
}

/// Runs the hierarchy migrations and builds its services. The subtree ports
/// call the agent module's own stop and delete use cases, so a subtree stop
/// or delete behaves exactly like the single-agent one (FR-006, FR-007). The
/// directive use case writes through the terminal runtime and journals into
/// the agent module's action record (FR-009, FR-011).
///
/// The hold service reads the stored holds and drops those of agents that
/// no longer exist (0007 D10).
///
/// # Errors
/// A migration failure, or holds that cannot be read.
#[allow(
    clippy::too_many_arguments,
    reason = "the hierarchy joins the agent, terminal, notification, and queue services"
)]
pub fn build_hierarchy_services(
    connection: &SharedConnection,
    agent: &AgentServices,
    terminal: &TerminalServices,
    alerts: &StorageAlerts,
    log: &ServerLog,
    events: &EventHub,
    notifications: &NotificationBridge,
    needs_you: &NeedsYouBridge,
) -> Result<HierarchyServices, StartupError> {
    let handle = connection.handle();
    let mut guard = handle
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // The hierarchy ledger guards no hand-edited data, so it needs no checks.
    run_migrations(&mut guard, LEDGER_TABLE, SCRIPTS, &[])?;
    drop(guard);
    let places = AgentPlaces::new();
    let repository = HierarchyRepository::new(connection.handle(), places);
    repair_parentless_leads(&repository, log);
    let store: Arc<dyn HierarchyStore> = Arc::new(repository);
    // The agent handlers and the hierarchy handlers each hold a subtree use
    // case over the same store and the same agent ports, so both answers
    // read the same tree and end agents the same way.
    let subtree = || {
        SubtreeHandler::new(
            Arc::clone(&store),
            Arc::new(StopAgentPort::new(&agent.handlers.use_cases().stop_agent)),
            Arc::new(DeleteAgentPort::new(
                &agent.handlers.use_cases().delete_agent,
            )),
        )
    };
    agent
        .handlers
        .with_subtree(Arc::new(SubtreeRequestsBridge::new(subtree())));
    // The role bridge is the `agent.create` route for a request that carries
    // a `role` object: the hierarchy's one-step create-and-place (0002,
    // FR-003). The creator port calls the agent module's own create and
    // delete use cases, so a seeded create answers exactly like
    // `agent.create` and a refused place leaves no agent behind.
    let creator: Arc<dyn AgentCreator> = Arc::new(AgentCreatorBridge::new(
        agent.handlers.use_cases().create_agent.clone(),
        agent.handlers.use_cases().delete_agent.clone(),
    ));
    agent.handlers.with_roles(Arc::new(RoleRequestsBridge::new(
        CreateRoleAgentHandler::new(Arc::clone(&store), Arc::clone(&creator)),
        Arc::clone(&store),
    )));
    let tree = Arc::new(TreeQuery::new(Arc::clone(&store)));
    let handlers = Arc::new(HierarchyHandlers::new(HierarchyUseCases {
        tree: Arc::clone(&tree),
        create_team: CreateTeamHandler::new(Arc::clone(&store)).with_creator(Arc::clone(&creator)),
        delete_team: DeleteTeamHandler::new(Arc::clone(&store)),
        place_agent: PlaceAgentHandler::new(Arc::clone(&store)),
        subtree: subtree(),
    }));
    let directive_journal = Arc::new(DirectiveJournalBridge::new(
        agent.journal.clone(),
        alerts.clone(),
    ));
    let agents: Arc<dyn AgentStore> = Arc::new(AgentRepository::new(connection.clone()));
    let holds = build_hold_service(
        connection,
        agent,
        alerts,
        log,
        events,
        notifications,
        needs_you,
    )?;
    prune_holds(&holds, agents.as_ref(), log);
    // One target bridge for directives and reports: both read whether the
    // receiver runs and what it is called the same way.
    let targets: Arc<dyn DirectiveTarget> = Arc::new(DirectiveTargetBridge::new(
        Arc::clone(&agents),
        Arc::clone(&terminal.runtime),
        terminal.person_input.clone(),
    ));
    let send_directive = Arc::new(
        SendDirectiveHandler::new(DirectivePorts {
            store: Arc::clone(&store),
            targets: Arc::clone(&targets),
            writer: Arc::new(DirectiveWriterBridge::new(
                Arc::clone(&terminal.runtime),
                terminal.person_input.clone(),
            )),
            journal: Arc::clone(&directive_journal) as Arc<dyn DirectiveJournal>,
            receipt: Arc::new(DirectiveReceiptBridge::new(
                agents,
                events.clone(),
                alerts.clone(),
                Arc::clone(&agent.task_order),
            )),
            clock: Arc::new(SteadyClock::default()),
        })
        .with_hold_guard(Arc::clone(&holds))
        .with_report_journal(Arc::clone(&directive_journal) as Arc<dyn ReportJournal>),
    );
    spawn_directive_delivery(&send_directive, events.clone());
    let send_report = Arc::new(
        SendReportHandler::new(ReportPorts {
            store: Arc::clone(&store),
            targets,
            journal: Arc::clone(&directive_journal) as Arc<dyn ReportJournal>,
            person: Arc::new(PersonReportsBridge::new(notifications.clone())),
            parent_delivery: Arc::clone(&send_directive),
        })
        .with_hold_guard(Arc::clone(&holds)),
    );
    Ok(HierarchyServices {
        handlers,
        tree,
        send_directive,
        send_report,
        directive_journal,
        holds,
        store,
        creator,
    })
}

fn build_hold_service(
    connection: &SharedConnection,
    agent: &AgentServices,
    alerts: &StorageAlerts,
    log: &ServerLog,
    events: &EventHub,
    notifications: &NotificationBridge,
    needs_you: &NeedsYouBridge,
) -> Result<Arc<HoldService>, StartupError> {
    let store: Arc<dyn HoldStore> = Arc::new(HoldRepository::new(connection.handle()));
    let limits: Arc<dyn RateLimitSource> = Arc::new(SettingsRateLimits::new(
        agent.handlers.use_cases().get_settings.clone(),
        log.clone(),
    ));
    let journal: Arc<dyn HoldJournal> = Arc::new(HoldJournalBridge::new(
        agent.record_action.clone(),
        needs_you.clone(),
        events.clone(),
        alerts.clone(),
    ));
    let notifier: Arc<dyn HoldNotifier> = Arc::new(HoldNotifierBridge::new(
        notifications.clone(),
        Arc::clone(&store),
        events.clone(),
        alerts.clone(),
    ));
    let service = HoldService::new(
        store,
        limits,
        journal,
        notifier,
        Arc::new(SteadyClock::default()) as Arc<dyn HoldClock>,
    )
    .map_err(|error| StartupError::Holds(error.to_string()))?;
    Ok(Arc::new(service))
}

/// An office saved before 0002 A5 was fixed can hold a CEO and leads with
/// no parent; they move under the CEO at start. A failure is logged and
/// leaves the tree as it was, because the tree still shows those teams.
fn repair_parentless_leads(repository: &HierarchyRepository, log: &ServerLog) {
    match repository.repair_parentless_leads() {
        Ok(0) => {}
        Ok(moved) => log.info(&format!("{moved} team leads moved under the CEO (0002 A5)")),
        Err(error) => log.error(&format!(
            "the team leads were not moved under the CEO: {error}"
        )),
    }
}

/// A hold of an agent that was deleted while the server was down would never
/// show in the panel; it goes at start. A failed read or write leaves the
/// holds as they are and is logged, because a stale hold only waits.
fn prune_holds(holds: &HoldService, agents: &dyn AgentStore, log: &ServerLog) {
    let existing = match agents.list() {
        Ok(listed) => listed.iter().map(|agent| agent.id).collect(),
        Err(error) => return log.error(&format!("the stale holds were not pruned: {error}")),
    };
    if let Err(error) = holds.prune(&existing) {
        log.error(&format!("the stale holds were not pruned: {error}"));
    }
}

/// Bridges one agent stop to the agent module's own handler.
struct StopAgentPort {
    stop: StopAgentHandler,
}

impl StopAgentPort {
    fn new(stop: &StopAgentHandler) -> Self {
        Self { stop: stop.clone() }
    }
}

impl SubtreeAgentEnder for StopAgentPort {
    fn stop(&self, agent_id: AgentId) -> Result<(), SubtreeActionError> {
        self.stop
            .handle(agent_id)
            .map(|_| ())
            .map_err(|error| SubtreeActionError(error.to_string()))
    }
}

/// Bridges one agent delete to the agent module's own handler.
struct DeleteAgentPort {
    delete: DeleteAgentHandler,
}

impl DeleteAgentPort {
    fn new(delete: &DeleteAgentHandler) -> Self {
        Self {
            delete: delete.clone(),
        }
    }
}

impl SubtreeAgentRemover for DeleteAgentPort {
    fn check(
        &self,
        agent_id: AgentId,
        worktrees: &SubtreeWorktrees,
    ) -> Result<(), SubtreeActionError> {
        self.delete
            .check(agent_id, &delete_worktrees(worktrees))
            .map_err(|error| SubtreeActionError(error.to_string()))
    }

    fn prepare(
        &self,
        agent_id: AgentId,
        worktrees: &SubtreeWorktrees,
    ) -> Result<(), SubtreeActionError> {
        self.delete
            .stop_and_check(agent_id, &delete_worktrees(worktrees))
            .map_err(|error| SubtreeActionError(error.to_string()))
    }

    fn delete(
        &self,
        agent_id: AgentId,
        worktrees: &SubtreeWorktrees,
    ) -> Result<WorktreeReport, SubtreeActionError> {
        self.delete
            .handle(agent_id, &delete_worktrees(worktrees))
            .map(worktree_report)
            .map_err(|error| SubtreeActionError(error.to_string()))
    }
}
