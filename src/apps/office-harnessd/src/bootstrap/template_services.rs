// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_application::common::interfaces::{AdditionsStore, AgentStore, ProjectStore};
use agent_infrastructure::persistence::{AdditionsRepository, ProjectRepository, SharedConnection};
use template_application::common::interfaces::{AgentSource, TemplateClock, TemplateStore};
use template_application::services::TemplateService;
use template_infrastructure::persistence::TemplateRepository;
use template_presentation::TemplateHandlers;

use super::HierarchyServices;
use crate::bridges::{AgentDefinitionBridge, SystemClock, TemplateCreateBridge};

/// The template services of the host: the template use cases and handlers,
/// and the create bridge, which joins the template store to the agent and
/// hierarchy modules.
pub struct TemplateServices {
    pub handlers: Arc<TemplateHandlers>,
    pub create: Arc<TemplateCreateBridge>,
    /// The stores the bridge reads, shared with the template use cases.
    pub templates: Arc<dyn TemplateStore>,
    pub projects: Arc<dyn ProjectStore>,
}

/// Builds the template services over the one database connection. The
/// create bridge shares the hierarchy store and the agent creator, so its
/// writes take the same tree exclusion as every other tree writer.
#[must_use]
pub fn build_template_services(
    connection: &SharedConnection,
    agents: Arc<dyn AgentStore>,
    hierarchy: &HierarchyServices,
) -> TemplateServices {
    let templates: Arc<dyn TemplateStore> = Arc::new(TemplateRepository::new(connection.handle()));
    let projects: Arc<dyn ProjectStore> = Arc::new(ProjectRepository::new(connection.clone()));
    let additions: Arc<dyn AdditionsStore> = Arc::new(AdditionsRepository::new(connection.clone()));
    let definitions: Arc<dyn AgentSource> = Arc::new(AgentDefinitionBridge::new(
        Arc::clone(&agents),
        additions,
        Arc::clone(&hierarchy.store),
    ));
    let clock: Arc<dyn TemplateClock> = Arc::new(SystemClock);
    let service = TemplateService::new(Arc::clone(&templates), definitions, clock);
    TemplateServices {
        handlers: Arc::new(TemplateHandlers::new(service)),
        create: Arc::new(TemplateCreateBridge::new(
            Arc::clone(&templates),
            Arc::clone(&hierarchy.store),
            Arc::clone(&hierarchy.creator),
            agents,
            Arc::clone(&projects),
        )),
        templates,
        projects,
    }
}
