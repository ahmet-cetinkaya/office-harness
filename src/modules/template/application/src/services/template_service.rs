// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;
use std::sync::Arc;

use kernel::AgentId;
use template_domain::{
    AgentBody, FileTemplate, NamePattern, Template, TemplateBody, TemplateId, TemplateName,
    default_agent_pattern, parse_file, render_file, save_name,
};

use crate::common::errors::TemplateApplicationError;
use crate::common::interfaces::{AgentSource, TemplateClock, TemplateStore};
use crate::common::models::ImportPreviewEntry;

/// Saves, lists, and deletes templates, copies an agent's definition, and
/// moves templates through a file (FR-001, FR-003, FR-006 to FR-010). It
/// writes nothing to the action record: it has no port for it (A10).
#[derive(Clone)]
pub struct TemplateService {
    store: Arc<dyn TemplateStore>,
    agents: Arc<dyn AgentSource>,
    clock: Arc<dyn TemplateClock>,
}

impl TemplateService {
    #[must_use]
    pub fn new(
        store: Arc<dyn TemplateStore>,
        agents: Arc<dyn AgentSource>,
        clock: Arc<dyn TemplateClock>,
    ) -> Self {
        Self {
            store,
            agents,
            clock,
        }
    }

    /// # Errors
    /// `Storage`.
    pub fn list(&self) -> Result<Vec<Template>, TemplateApplicationError> {
        self.store.list()
    }

    /// # Errors
    /// `NotFound` for an unknown id, `Storage`.
    pub fn get(&self, id: TemplateId) -> Result<Template, TemplateApplicationError> {
        self.store.find(id)?.ok_or_else(|| not_found(id))
    }

    /// Creates a template, or replaces the one with `id` (D7). Agents and
    /// teams created from the old text stay as they are (FR-006).
    ///
    /// # Errors
    /// `Invalid` with the save rule that fails, `NameTaken`, `NotFound` for
    /// an unknown `id`, `Storage`.
    pub fn save(
        &self,
        id: Option<TemplateId>,
        name: &str,
        body: TemplateBody,
    ) -> Result<Template, TemplateApplicationError> {
        let name = TemplateName::parse(name)?;
        let template = Template::new(
            id.unwrap_or_else(TemplateId::generate),
            name,
            body,
            self.clock.now(),
        )?;
        if id.is_some() {
            self.store.update(&template)?;
        } else {
            self.store.insert(&template)?;
        }
        Ok(template)
    }

    /// # Errors
    /// `NotFound` for an unknown id, `Storage`.
    pub fn delete(&self, id: TemplateId) -> Result<(), TemplateApplicationError> {
        if self.store.delete(id)? {
            return Ok(());
        }
        Err(not_found(id))
    }

    /// An unsaved agent body copied from an agent, with the A2 pattern. The
    /// editor saves it.
    ///
    /// # Errors
    /// `NotFound` for an unknown agent, `Storage`.
    pub fn body_from_agent(
        &self,
        agent_id: AgentId,
    ) -> Result<AgentBody, TemplateApplicationError> {
        let definition = self.agents.definition_of(agent_id)?;
        Ok(AgentBody {
            name_pattern: NamePattern::new(default_agent_pattern(&definition.name)),
            command: definition.command,
            working_folder: definition.working_folder,
            additions: definition.additions,
            role: definition.role,
            own_worktree: definition.own_worktree,
        })
    }

    /// The file text of the templates in `ids`, or of all of them, with every
    /// environment value `null` (FR-007).
    ///
    /// # Errors
    /// `NotFound` for an unknown id, `Storage`.
    pub fn export(&self, ids: Option<&[TemplateId]>) -> Result<String, TemplateApplicationError> {
        let all = self.store.list()?;
        let chosen = match ids {
            None => all,
            Some(ids) => ids
                .iter()
                .map(|id| {
                    all.iter()
                        .find(|template| template.id == *id)
                        .cloned()
                        .ok_or_else(|| not_found(*id))
                })
                .collect::<Result<_, _>>()?,
        };
        let entries: Vec<FileTemplate> = chosen
            .into_iter()
            .map(|template| FileTemplate {
                name: template.name,
                body: template.body,
            })
            .collect();
        Ok(render_file(&entries))
    }

    /// What an import would save. It saves nothing (FR-008).
    ///
    /// # Errors
    /// `Invalid` with the FR-009 reason, naming the template; `Storage`.
    pub fn preview_import(
        &self,
        text: &str,
    ) -> Result<Vec<ImportPreviewEntry>, TemplateApplicationError> {
        let templates = parse_file(text)?;
        let mut taken_keys = self.existing_keys()?;
        Ok(templates
            .into_iter()
            .map(|template| {
                let save_name = save_name(&template.name, |key| taken_keys.contains(key));
                taken_keys.insert(save_name.key());
                ImportPreviewEntry {
                    name: template.name,
                    save_name,
                    body: template.body.without_env_values(),
                }
            })
            .collect())
    }

    /// Validates the text again and saves every template of it, with every
    /// environment value dropped, in one transaction. It creates no agent.
    ///
    /// # Errors
    /// `Invalid`, `NameTaken` when another save took a name meanwhile,
    /// `Storage`. Nothing is saved on an error.
    pub fn confirm_import(&self, text: &str) -> Result<Vec<Template>, TemplateApplicationError> {
        let now = self.clock.now();
        let templates = self
            .preview_import(text)?
            .into_iter()
            .map(|entry| Template::new(TemplateId::generate(), entry.save_name, entry.body, now))
            .collect::<Result<Vec<_>, _>>()?;
        self.store.insert_all(&templates)?;
        Ok(templates)
    }

    fn existing_keys(&self) -> Result<HashSet<String>, TemplateApplicationError> {
        Ok(self
            .store
            .list()?
            .iter()
            .map(|template| template.name.key())
            .collect())
    }
}

fn not_found(id: TemplateId) -> TemplateApplicationError {
    TemplateApplicationError::NotFound(format!("template {id} does not exist"))
}

#[cfg(test)]
#[path = "template_service.tests.rs"]
mod tests;
