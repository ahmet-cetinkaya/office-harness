// SPDX-License-Identifier: AGPL-3.0-or-later

//! The repositories as the stores that the agent use cases declare.

use agent_application::common::errors::PortError;
use agent_application::common::interfaces::{
    AdditionsStore, AgentStore, ProjectStore, SessionStore, SettingsStore,
};
use agent_domain::entities::{AdditionsRecord, Agent, Project, Session};
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{ExitResult, ProjectId, SessionId, Settings};
use kernel::{AgentId, Timestamp};

use crate::persistence::{
    AdditionsRepository, AgentRepository, PersistenceError, ProjectRepository, SessionRepository,
    SettingsRepository,
};

impl From<PersistenceError> for PortError {
    fn from(error: PersistenceError) -> Self {
        let is_disk_failure = error.is_storage_failure();
        match error {
            PersistenceError::Duplicate { what } => Self::Duplicate { what },
            PersistenceError::NotFound { what } => Self::NotFound { what },
            PersistenceError::ProjectInUse { project, agents } => {
                Self::ProjectInUse { project, agents }
            }
            PersistenceError::Invalid(_)
            | PersistenceError::InvalidAgentId(_)
            | PersistenceError::InvalidProjectId(_)
            | PersistenceError::InvalidJson(_) => Self::InvalidData {
                message: error.to_string(),
            },
            PersistenceError::Storage(_) => Self::Storage {
                message: error.to_string(),
                is_disk_failure,
            },
        }
    }
}

impl AgentStore for AgentRepository {
    fn insert(&self, agent: &Agent) -> Result<(), PortError> {
        Ok(Self::insert(self, agent)?)
    }

    fn find(&self, id: AgentId) -> Result<Option<Agent>, PortError> {
        Ok(Self::find(self, id)?)
    }

    fn list(&self) -> Result<Vec<Agent>, PortError> {
        Ok(Self::list(self)?)
    }

    fn update_status(&self, id: AgentId, status: AgentStatus) -> Result<(), PortError> {
        Ok(Self::update_status(self, id, status)?)
    }

    fn delete(&self, id: AgentId) -> Result<(), PortError> {
        Ok(Self::delete(self, id)?)
    }
}

impl ProjectStore for ProjectRepository {
    fn insert(&self, project: &Project) -> Result<(), PortError> {
        Ok(Self::insert(self, project)?)
    }

    fn find(&self, id: ProjectId) -> Result<Option<Project>, PortError> {
        Ok(Self::find(self, id)?)
    }

    fn list(&self) -> Result<Vec<Project>, PortError> {
        Ok(Self::list(self)?)
    }

    fn delete(&self, id: ProjectId) -> Result<(), PortError> {
        Ok(Self::delete(self, id)?)
    }
}

impl SettingsStore for SettingsRepository {
    fn load(&self) -> Result<Settings, PortError> {
        Ok(Self::load(self)?)
    }

    fn save(&self, settings: &Settings) -> Result<(), PortError> {
        Ok(Self::save(self, settings)?)
    }
}

impl AdditionsStore for AdditionsRepository {
    fn load(&self, agent_id: AgentId) -> Result<Option<AdditionsRecord>, PortError> {
        Ok(Self::load(self, agent_id)?)
    }

    fn save(&self, agent_id: AgentId, record: &AdditionsRecord) -> Result<(), PortError> {
        Ok(Self::save(self, agent_id, record)?)
    }
}

impl SessionStore for SessionRepository {
    fn insert(&self, session: &Session) -> Result<(), PortError> {
        Ok(Self::insert(self, session)?)
    }

    fn find(&self, id: SessionId) -> Result<Option<Session>, PortError> {
        Ok(Self::find(self, id)?)
    }

    fn list_for_agent(&self, agent_id: AgentId) -> Result<Vec<Session>, PortError> {
        Ok(Self::list_for_agent(self, agent_id)?)
    }

    fn latest_for_agent(&self, agent_id: AgentId) -> Result<Option<Session>, PortError> {
        Ok(Self::latest_for_agent(self, agent_id)?)
    }

    fn set_cli_session_id(&self, id: SessionId, cli_session_id: &str) -> Result<(), PortError> {
        Ok(Self::set_cli_session_id(self, id, cli_session_id)?)
    }

    fn end_running(
        &self,
        agent_id: AgentId,
        result: &ExitResult,
        at: Timestamp,
    ) -> Result<bool, PortError> {
        Ok(Self::end_running(self, agent_id, result, at)?)
    }
}

#[cfg(test)]
#[path = "port_adapters.tests.rs"]
mod tests;
