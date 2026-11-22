// SPDX-License-Identifier: AGPL-3.0-or-later

//! In-memory fakes of the ports, for the use case tests.

use std::sync::{Arc, Mutex, PoisonError};

use kernel::{AgentId, Timestamp};
use template_domain::{
    AdditionsBody, AgentBody, EnvVariable, NamePattern, Template, TemplateBody, TemplateId,
};

use crate::common::errors::TemplateApplicationError;
use crate::common::interfaces::{AgentSource, TemplateClock, TemplateStore};
use crate::common::models::AgentDefinition;
use crate::services::TemplateService;

#[derive(Default)]
pub struct MemoryStore {
    rows: Mutex<Vec<Template>>,
}

impl MemoryStore {
    fn rows(&self) -> std::sync::MutexGuard<'_, Vec<Template>> {
        self.rows.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn check_name_free(
        rows: &[Template],
        template: &Template,
    ) -> Result<(), TemplateApplicationError> {
        let taken = rows
            .iter()
            .any(|row| row.id != template.id && row.name.key() == template.name.key());
        if taken {
            return Err(TemplateApplicationError::NameTaken {
                name: template.name.to_string(),
            });
        }
        Ok(())
    }
}

impl TemplateStore for MemoryStore {
    fn list(&self) -> Result<Vec<Template>, TemplateApplicationError> {
        let mut rows = self.rows().clone();
        rows.sort_by_key(|row| row.name.key());
        Ok(rows)
    }

    fn find(&self, id: TemplateId) -> Result<Option<Template>, TemplateApplicationError> {
        Ok(self.rows().iter().find(|row| row.id == id).cloned())
    }

    fn insert(&self, template: &Template) -> Result<(), TemplateApplicationError> {
        self.insert_all(std::slice::from_ref(template))
    }

    fn update(&self, template: &Template) -> Result<(), TemplateApplicationError> {
        let mut rows = self.rows();
        Self::check_name_free(&rows, template)?;
        let row = rows
            .iter_mut()
            .find(|row| row.id == template.id)
            .ok_or_else(|| TemplateApplicationError::NotFound("missing".to_owned()))?;
        *row = template.clone();
        Ok(())
    }

    fn delete(&self, id: TemplateId) -> Result<bool, TemplateApplicationError> {
        let mut rows = self.rows();
        let before = rows.len();
        rows.retain(|row| row.id != id);
        Ok(rows.len() != before)
    }

    fn insert_all(&self, templates: &[Template]) -> Result<(), TemplateApplicationError> {
        let mut rows = self.rows();
        let mut next = rows.clone();
        for template in templates {
            Self::check_name_free(&next, template)?;
            next.push(template.clone());
        }
        *rows = next;
        Ok(())
    }
}

pub struct FakeAgents(pub AgentDefinition);

impl AgentSource for FakeAgents {
    fn definition_of(&self, _: AgentId) -> Result<AgentDefinition, TemplateApplicationError> {
        Ok(self.0.clone())
    }
}

pub struct FixedClock;

impl TemplateClock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_millis(1_700_000_000_000)
    }
}

pub fn agent_body(pattern: &str) -> AgentBody {
    AgentBody {
        name_pattern: NamePattern::new(pattern),
        command: "claude".to_owned(),
        working_folder: None,
        additions: AdditionsBody {
            flags: Vec::new(),
            env: vec![EnvVariable {
                name: "API_KEY".to_owned(),
                value: Some("secret".to_owned()),
            }],
            startup_prompt: None,
        },
        role: None,
        own_worktree: false,
    }
}

pub fn agent_template_body(pattern: &str) -> TemplateBody {
    TemplateBody::Agent(agent_body(pattern))
}

pub fn definition() -> AgentDefinition {
    AgentDefinition {
        name: "Ana".to_owned(),
        command: "claude --model opus".to_owned(),
        working_folder: Some("/work".to_owned()),
        additions: agent_body("x").additions,
        role: None,
        own_worktree: false,
    }
}

pub fn service() -> TemplateService {
    TemplateService::new(
        Arc::new(MemoryStore::default()),
        Arc::new(FakeAgents(definition())),
        Arc::new(FixedClock),
    )
}
