// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_application::common::interfaces::{AdditionsStore, AgentStore};
use hierarchy_application::common::interfaces::HierarchyStore;
use hierarchy_domain::Role;
use kernel::AgentId;
use template_application::common::errors::TemplateApplicationError;
use template_application::common::interfaces::AgentSource;
use template_application::common::models::AgentDefinition;
use template_domain::{AdditionsBody, EnvVariable, TemplateRole};

/// Reads an agent's definition for the template module (0009 FR-001): the
/// agent module gives the name, command, folder, and office additions, and
/// the hierarchy gives the role. The template module names neither module.
pub struct AgentDefinitionBridge {
    agents: Arc<dyn AgentStore>,
    additions: Arc<dyn AdditionsStore>,
    hierarchy: Arc<dyn HierarchyStore>,
}

impl AgentDefinitionBridge {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        additions: Arc<dyn AdditionsStore>,
        hierarchy: Arc<dyn HierarchyStore>,
    ) -> Self {
        Self {
            agents,
            additions,
            hierarchy,
        }
    }

    fn additions_of(&self, agent_id: AgentId) -> Result<AdditionsBody, TemplateApplicationError> {
        let record = self
            .additions
            .load(agent_id)
            .map_err(|error| storage_failure(&error))?;
        Ok(record.map_or_else(AdditionsBody::default, |record| {
            let additions = record.additions;
            AdditionsBody {
                flags: additions.flags().to_vec(),
                env: additions
                    .env()
                    .iter()
                    .map(|entry| EnvVariable {
                        name: entry.name().to_owned(),
                        value: entry.value().map(str::to_owned),
                    })
                    .collect(),
                startup_prompt: additions.startup_prompt().map(str::to_owned),
            }
        }))
    }

    fn role_of(&self, agent_id: AgentId) -> Result<Option<TemplateRole>, TemplateApplicationError> {
        let snapshot = self
            .hierarchy
            .snapshot()
            .map_err(|error| storage_failure(&error))?;
        Ok(snapshot
            .rows
            .iter()
            .find(|row| row.agent_id == agent_id)
            .and_then(|row| row.role)
            .map(template_role))
    }
}

impl AgentSource for AgentDefinitionBridge {
    fn definition_of(
        &self,
        agent_id: AgentId,
    ) -> Result<AgentDefinition, TemplateApplicationError> {
        let agent = self
            .agents
            .find(agent_id)
            .map_err(|error| storage_failure(&error))?
            .ok_or_else(|| {
                TemplateApplicationError::NotFound(format!("agent {agent_id} does not exist"))
            })?;
        Ok(AgentDefinition {
            name: agent.name.as_str().to_owned(),
            command: agent.command.as_str().to_owned(),
            working_folder: agent
                .working_folder
                .map(|folder| folder.to_string_lossy().into_owned()),
            additions: self.additions_of(agent_id)?,
            role: self.role_of(agent_id)?,
        })
    }
}

fn template_role(role: Role) -> TemplateRole {
    match role {
        Role::Ceo => TemplateRole::Ceo,
        Role::Lead => TemplateRole::Lead,
        Role::Member => TemplateRole::Member,
    }
}

/// A failed read of another module's store, as the template module carries
/// it. Whether the disk is at fault is not known here, so no storage alert
/// is raised for it.
fn storage_failure(error: &impl std::fmt::Display) -> TemplateApplicationError {
    TemplateApplicationError::Storage {
        message: error.to_string(),
        is_disk_failure: false,
    }
}
