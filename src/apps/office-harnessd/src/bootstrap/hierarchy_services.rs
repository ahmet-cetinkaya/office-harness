// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_application::commands::delete_agent::DeleteAgentHandler;
use agent_application::commands::stop_agent::StopAgentHandler;
use agent_infrastructure::persistence::SharedConnection;
use hierarchy_application::commands::subtree::SubtreeHandler;
use hierarchy_application::common::ports::{
    SubtreeActionError, SubtreeAgentEnder, SubtreeAgentRemover,
};
use hierarchy_application::{CreateTeamHandler, DeleteTeamHandler, PlaceAgentHandler, TreeQuery};
use hierarchy_infrastructure::persistence::migrations::{LEDGER_TABLE, SCRIPTS};
use hierarchy_infrastructure::persistence::repositories::hierarchy_repository::HierarchyRepository;
use hierarchy_infrastructure::services::agent_places::AgentPlaces;
use hierarchy_presentation::{HierarchyHandlers, HierarchyUseCases};
use kernel::AgentId;

use super::{AgentServices, ServerLog, StartupError, StorageAlerts, run_migrations};
use crate::bridges::SubtreeRequestsBridge;

/// The hierarchy services: the store, the use cases, and the handlers.
pub struct HierarchyServices {
    pub handlers: Arc<HierarchyHandlers>,
}

/// Runs the hierarchy migrations and builds its services. The subtree ports
/// call the agent module's own stop and delete use cases, so a subtree stop
/// or delete behaves exactly like the single-agent one (FR-006, FR-007).
///
/// # Errors
/// A migration failure.
pub fn build_hierarchy_services(
    connection: &SharedConnection,
    agent: &AgentServices,
    _alerts: &StorageAlerts,
    _log: &ServerLog,
) -> Result<HierarchyServices, StartupError> {
    let handle = connection.handle();
    let mut guard = handle
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // The hierarchy ledger guards no hand-edited data, so it needs no checks.
    run_migrations(&mut guard, LEDGER_TABLE, SCRIPTS, &[])?;
    drop(guard);
    let places = AgentPlaces::new();
    let store: Arc<dyn hierarchy_application::common::interfaces::HierarchyStore> =
        Arc::new(HierarchyRepository::new(connection.handle(), places));
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
    let handlers = Arc::new(HierarchyHandlers::new(HierarchyUseCases {
        tree: Arc::new(TreeQuery::new(Arc::clone(&store))),
        create_team: CreateTeamHandler::new(Arc::clone(&store)),
        delete_team: DeleteTeamHandler::new(Arc::clone(&store)),
        place_agent: PlaceAgentHandler::new(Arc::clone(&store)),
        subtree: subtree(),
    }));
    Ok(HierarchyServices { handlers })
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
    fn delete(&self, agent_id: AgentId) -> Result<(), SubtreeActionError> {
        self.delete
            .handle(agent_id)
            .map_err(|error| SubtreeActionError(error.to_string()))
    }
}
