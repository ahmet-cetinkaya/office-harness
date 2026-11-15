// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use agent_application::commands::create_agent::{
    CreateAgentHandler, CreateAgentRequest as CreateAgent,
};
use agent_application::commands::create_project::{
    CreateProjectHandler, CreateProjectRequest as CreateProject,
};
use agent_application::commands::delete_agent::DeleteAgentHandler;
use agent_application::commands::delete_project::DeleteProjectHandler;
use agent_application::commands::set_setting::SetSettingHandler;
use agent_application::commands::start_agent::StartAgentHandler;
use agent_application::commands::stop_agent::StopAgentHandler;
use agent_application::queries::get_settings::GetSettingsHandler;
use agent_application::queries::list_agents::ListAgentsHandler;
use agent_application::queries::list_projects::ListProjectsHandler;
use agent_domain::value_objects::{ProjectId, SettingKey};
use kernel::AgentId;
use serde_json::Value;
use wire::{Event, Request, Response};

use super::handler_error::HandlerError;
use crate::contracts::{
    AgentIdRequest, CreateAgentRequest, CreateProjectRequest, ProjectIdRequest, SettingKeyRequest,
    SettingRequest,
};
use crate::mappings::{
    agent_changed_event, agent_deleted_event, agent_json, agent_status_event, project_json,
    setting_json,
};

/// The host sends every request whose type starts with one of these to
/// `AgentHandlers`, except `agent.actions`.
pub const REQUEST_PREFIXES: [&str; 3] = ["agent.", "project.", "settings."];

/// The use cases that the handlers call.
pub struct AgentUseCases {
    pub create_agent: CreateAgentHandler,
    pub delete_agent: DeleteAgentHandler,
    pub start_agent: StartAgentHandler,
    pub stop_agent: StopAgentHandler,
    pub list_agents: ListAgentsHandler,
    pub create_project: CreateProjectHandler,
    pub delete_project: DeleteProjectHandler,
    pub list_projects: ListProjectsHandler,
    pub set_setting: SetSettingHandler,
    pub get_settings: GetSettingsHandler,
}

/// The answer to one request, the events that every client gets because of
/// it, and the disk failure text when the disk is at fault.
#[derive(Debug)]
pub struct Handled {
    pub response: Response,
    pub events: Vec<Event>,
    pub disk_failure: Option<String>,
}

struct Success {
    result: Value,
    events: Vec<Event>,
}

impl Success {
    fn result(result: Value) -> Self {
        Self {
            result,
            events: Vec::new(),
        }
    }

    fn done() -> Self {
        Self::result(Value::Null)
    }

    fn with_event(mut self, event: Event) -> Self {
        self.events.push(event);
        self
    }
}

/// The handlers of the agent, project, and settings requests. Each one parses
/// its body, names the field at fault, and calls the use case. The handlers
/// block, so the host runs them on a blocking thread.
pub struct AgentHandlers {
    use_cases: AgentUseCases,
}

impl AgentHandlers {
    #[must_use]
    pub fn new(use_cases: AgentUseCases) -> Self {
        Self { use_cases }
    }

    #[must_use]
    pub fn handle(&self, request: &Request) -> Handled {
        let id = request.id;
        match self.route(request) {
            Ok(success) => Handled {
                response: Response::success(id, success.result),
                events: success.events,
                disk_failure: None,
            },
            Err(error) => Handled {
                response: Response::failure(id, error.code, error.message),
                events: Vec::new(),
                disk_failure: error.disk_failure,
            },
        }
    }

    fn route(&self, request: &Request) -> Result<Success, HandlerError> {
        match request.kind.as_str() {
            "project.list" => self.list_projects(),
            "project.create" => self.create_project(request),
            "project.delete" => self.delete_project(request),
            "agent.list" => self.list_agents(),
            "agent.create" => self.create_agent(request),
            "agent.delete" => self.delete_agent(request),
            "agent.start" => self.start_agent(request),
            "agent.stop" => self.stop_agent(request),
            "settings.get" => self.get_setting(request),
            "settings.set" => self.set_setting(request),
            other => Err(HandlerError::not_found(format!(
                "no request of the type {other}"
            ))),
        }
    }

    fn list_projects(&self) -> Result<Success, HandlerError> {
        let projects = self.use_cases.list_projects.handle()?;
        let projects = projects.iter().map(project_json).collect();
        Ok(Success::result(Value::Array(projects)))
    }

    fn create_project(&self, request: &Request) -> Result<Success, HandlerError> {
        let body = CreateProjectRequest::read(&request.fields).map_err(HandlerError::invalid)?;
        let project = self.use_cases.create_project.handle(CreateProject {
            name: body.name,
            folder: PathBuf::from(body.folder),
        })?;
        Ok(Success::result(project_json(&project)))
    }

    fn delete_project(&self, request: &Request) -> Result<Success, HandlerError> {
        let body = ProjectIdRequest::read(&request.fields).map_err(HandlerError::invalid)?;
        let project_id: ProjectId = parse_id("project_id", &body.project_id)?;
        self.use_cases.delete_project.handle(project_id)?;
        Ok(Success::done())
    }

    fn list_agents(&self) -> Result<Success, HandlerError> {
        let agents = self.use_cases.list_agents.handle()?;
        let agents = agents.iter().map(agent_json).collect();
        Ok(Success::result(Value::Array(agents)))
    }

    fn create_agent(&self, request: &Request) -> Result<Success, HandlerError> {
        let body = CreateAgentRequest::read(&request.fields).map_err(HandlerError::invalid)?;
        let project_id = body
            .project_id
            .as_deref()
            .map(|text| parse_id::<ProjectId>("project_id", text))
            .transpose()?;
        let agent = self.use_cases.create_agent.handle(CreateAgent {
            name: body.name,
            command: body.command,
            working_folder: body.working_folder.map(PathBuf::from),
            project_id,
        })?;
        Ok(Success::result(agent_json(&agent)).with_event(agent_changed_event(&agent)))
    }

    fn delete_agent(&self, request: &Request) -> Result<Success, HandlerError> {
        let agent_id = parse_agent_id(request)?;
        self.use_cases.delete_agent.handle(agent_id)?;
        Ok(Success::done().with_event(agent_deleted_event(agent_id)))
    }

    fn start_agent(&self, request: &Request) -> Result<Success, HandlerError> {
        let agent_id = parse_agent_id(request)?;
        let outcome = self.use_cases.start_agent.handle(agent_id)?;
        let status = outcome.agent.status;
        let detail = outcome.failure.as_deref();
        let mut result = serde_json::json!({
            "agent_id": agent_id.to_string(),
            "status": status.as_str(),
        });
        if let Some(detail) = detail {
            result["detail"] = Value::String(detail.to_owned());
        }
        Ok(Success::result(result).with_event(agent_status_event(agent_id, status, detail)))
    }

    fn stop_agent(&self, request: &Request) -> Result<Success, HandlerError> {
        let agent_id = parse_agent_id(request)?;
        let agent = self.use_cases.stop_agent.handle(agent_id)?;
        Ok(Success::done().with_event(agent_status_event(agent_id, agent.status, None)))
    }

    fn get_setting(&self, request: &Request) -> Result<Success, HandlerError> {
        let body = SettingKeyRequest::read(&request.fields).map_err(HandlerError::invalid)?;
        let key = SettingKey::parse(&body.key)
            .map_err(|error| HandlerError::invalid(error.to_string()))?;
        let settings = self.use_cases.get_settings.handle()?;
        Ok(Success::result(setting_json(
            key.as_str(),
            &settings.value_of(key),
        )))
    }

    fn set_setting(&self, request: &Request) -> Result<Success, HandlerError> {
        let body = SettingRequest::read(&request.fields).map_err(HandlerError::invalid)?;
        let settings = self.use_cases.set_setting.handle(&body.key, &body.value)?;
        let key = SettingKey::parse(&body.key)
            .map_err(|error| HandlerError::invalid(error.to_string()))?;
        Ok(Success::result(setting_json(
            key.as_str(),
            &settings.value_of(key),
        )))
    }
}

fn parse_agent_id(request: &Request) -> Result<AgentId, HandlerError> {
    let body = AgentIdRequest::read(&request.fields).map_err(HandlerError::invalid)?;
    parse_id("agent_id", &body.agent_id)
}

fn parse_id<T: std::str::FromStr>(field: &str, text: &str) -> Result<T, HandlerError> {
    text.parse()
        .map_err(|_| HandlerError::invalid(format!("{field} is not a valid identifier: {text}")))
}
