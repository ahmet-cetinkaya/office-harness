// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use agent_application::commands::continue_session::ContinueSessionHandler;
use agent_application::commands::create_agent::{
    CreateAgentHandler, CreateAgentRequest as CreateAgent,
};
use agent_application::commands::create_project::{
    CreateProjectHandler, CreateProjectRequest as CreateProject,
};
use agent_application::commands::delete_agent::DeleteAgentHandler;
use agent_application::commands::delete_project::DeleteProjectHandler;
use agent_application::commands::set_office_additions::SetOfficeAdditionsHandler;
use agent_application::commands::set_setting::SetSettingHandler;
use agent_application::commands::start_agent::{StartAgentHandler, StartOutcome};
use agent_application::commands::start_new_task::StartNewTaskHandler;
use agent_application::commands::stop_agent::StopAgentHandler;
use agent_application::common::continuation::fresh_session_view;
use agent_application::queries::get_office_additions::GetOfficeAdditionsHandler;
use agent_application::queries::get_settings::GetSettingsHandler;
use agent_application::queries::list_agents::ListAgentsHandler;
use agent_application::queries::list_projects::ListProjectsHandler;
use agent_application::queries::list_sessions::ListSessionsHandler;
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{ProjectId, SessionId, SettingKey};
use kernel::AgentId;
use serde_json::Value;
use wire::{Event, Request, Response};

use super::handler_error::HandlerError;
use super::role_requests::{RoleRequests, RoleResult};
use super::subtree_requests::{SubtreeRequests, SubtreeResult};
use crate::contracts::{
    AdditionsRequest, AgentIdRequest, CreateAgentRequest, CreateProjectRequest, ProjectIdRequest,
    RolePlacement, SessionIdRequest, SettingKeyRequest, SettingRequest,
};
use crate::mappings::{
    additions_changed_event, additions_json, agent_changed_event, agent_deleted_event, agent_json,
    agent_status_event, project_json, session_json, setting_json,
};

/// The host sends every request whose type starts with one of these to
/// `AgentHandlers`, except `agent.actions`.
pub const REQUEST_PREFIXES: [&str; 4] = ["agent.", "project.", "session.", "settings."];

/// The use cases that the handlers call.
pub struct AgentUseCases {
    pub create_agent: CreateAgentHandler,
    pub delete_agent: DeleteAgentHandler,
    pub start_agent: StartAgentHandler,
    pub stop_agent: StopAgentHandler,
    pub start_new_task: StartNewTaskHandler,
    pub continue_session: ContinueSessionHandler,
    pub set_office_additions: SetOfficeAdditionsHandler,
    pub get_office_additions: GetOfficeAdditionsHandler,
    pub list_sessions: ListSessionsHandler,
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
    /// The hierarchy's subtree bridge, when the host attaches it (0002).
    /// Absent keeps the single-agent behavior of 0001.
    subtree: OnceLock<Arc<dyn SubtreeRequests>>,
    /// The hierarchy's role bridge, when the host attaches it (0002). Absent
    /// refuses a `role` object, because no use case could place the agent.
    roles: OnceLock<Arc<dyn RoleRequests>>,
}

impl AgentHandlers {
    /// The use cases, for the host to bridge into other modules (the
    /// hierarchy's subtree stop and delete).
    #[must_use]
    pub fn use_cases(&self) -> &AgentUseCases {
        &self.use_cases
    }
}

impl AgentHandlers {
    #[must_use]
    pub fn new(use_cases: AgentUseCases) -> Self {
        Self {
            use_cases,
            subtree: OnceLock::new(),
            roles: OnceLock::new(),
        }
    }

    /// Attaches the hierarchy's subtree bridge (0002), so `agent.stop` and
    /// `agent.delete` accept `include_descendants`. Returns false when a
    /// bridge is already attached: the first attach wins, nothing is
    /// replaced.
    #[must_use]
    fn attach_subtree(&self, subtree: Arc<dyn SubtreeRequests>) -> bool {
        self.subtree.set(subtree).is_ok()
    }

    /// The host attaches the bridge once, when the handlers are already
    /// shared with the gateways. A second attach would wire a second bridge
    /// over the first, so a debug build names it a bug.
    pub fn with_subtree(&self, subtree: Arc<dyn SubtreeRequests>) {
        // First attach wins; the attach runs in every build, only the
        // panic is debug-only (a `debug_assert!` would skip the call in
        // release).
        let attached = self.attach_subtree(subtree);
        debug_assert!(attached, "the subtree bridge is attached twice");
    }

    /// Attaches the hierarchy's role bridge (0002), so `agent.create` with a
    /// `role` object creates and places in one step.
    pub fn with_roles(&self, roles: Arc<dyn RoleRequests>) {
        let attached = self.roles.set(roles).is_ok();
        debug_assert!(attached, "the role bridge is attached twice");
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
            "agent.new_task" => self.start_new_task(request),
            "agent.additions.get" => self.get_additions(request),
            "agent.additions.set" => self.set_additions(request),
            "session.list" => self.list_sessions(request),
            "session.continue" => self.continue_session(request),
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
        if let Some(role) = body.role.as_ref() {
            return self.create_role_agent(&body, role);
        }
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
            additions: body.additions,
        })?;
        Ok(Success::result(agent_json(&agent)).with_event(agent_changed_event(&agent)))
    }

    /// `agent.create` with a `role` object (0002, FR-003): the hierarchy's
    /// bridge creates and places in one step. Without the bridge there is
    /// nothing that could place the agent, so the request is refused and
    /// creates nothing (a plain no-role create would break the tree rules).
    fn create_role_agent(
        &self,
        body: &CreateAgentRequest,
        role: &RolePlacement,
    ) -> Result<Success, HandlerError> {
        let Some(roles) = self.roles.get() else {
            return Err(HandlerError::with_code(
                wire::ErrorCode::Refused,
                "role is not available: no hierarchy is attached",
            ));
        };
        let seed = super::role_requests::RoleSeed {
            name: body.name.clone(),
            command: body.command.clone(),
            working_folder: body.working_folder.clone(),
            project_id: body.project_id.clone(),
            additions: body.additions.clone(),
        };
        match roles.create_role_agent(
            &seed,
            &role.kind,
            role.parent_agent_id.clone(),
            role.team_id.clone(),
        ) {
            RoleResult::Created(agent_id) => {
                // The created agent reads back through the store the agent
                // module owns, so the answer is the record `agent.create`
                // always shows.
                let agent_id = parse_id::<AgentId>("agent_id", &agent_id)?;
                let agent = self.use_cases.list_agents.handle_one(agent_id)?;
                Ok(Success::result(agent_json(&agent)).with_event(agent_changed_event(&agent)))
            }
            RoleResult::Failed { code, message } => Err(HandlerError::with_code(code, message)),
        }
    }

    fn delete_agent(&self, request: &Request) -> Result<Success, HandlerError> {
        let agent_id = parse_agent_id(request)?;
        if let Some(subtree) = self.subtree.get() {
            return subtree_delete(subtree.as_ref(), request, agent_id);
        }
        self.use_cases.delete_agent.handle(agent_id)?;
        Ok(Success::done().with_event(agent_deleted_event(agent_id)))
    }

    fn start_agent(&self, request: &Request) -> Result<Success, HandlerError> {
        let agent_id = parse_agent_id(request)?;
        let outcome = self.use_cases.start_agent.handle(agent_id)?;
        Ok(started_session(outcome))
    }

    fn start_new_task(&self, request: &Request) -> Result<Success, HandlerError> {
        let agent_id = parse_agent_id(request)?;
        let outcome = self.use_cases.start_new_task.handle(agent_id)?;
        Ok(started_session(outcome))
    }

    fn continue_session(&self, request: &Request) -> Result<Success, HandlerError> {
        let body = SessionIdRequest::read(&request.fields).map_err(HandlerError::invalid)?;
        let session_id: SessionId = parse_id("session_id", &body.session_id)?;
        let outcome = self.use_cases.continue_session.handle(session_id)?;
        Ok(started_session(outcome))
    }

    fn list_sessions(&self, request: &Request) -> Result<Success, HandlerError> {
        let agent_id = parse_agent_id(request)?;
        let views = self.use_cases.list_sessions.handle(agent_id)?;
        let sessions = views.iter().map(session_json).collect();
        Ok(Success::result(Value::Array(sessions)))
    }

    fn get_additions(&self, request: &Request) -> Result<Success, HandlerError> {
        let agent_id = parse_agent_id(request)?;
        let view = self.use_cases.get_office_additions.handle(agent_id)?;
        Ok(Success::result(additions_json(&view)))
    }

    fn set_additions(&self, request: &Request) -> Result<Success, HandlerError> {
        let body = AdditionsRequest::read(&request.fields).map_err(HandlerError::invalid)?;
        let agent_id: AgentId = parse_id("agent_id", &body.agent_id)?;
        let view = self
            .use_cases
            .set_office_additions
            .handle(agent_id, body.additions)?;
        Ok(Success::result(additions_json(&view))
            .with_event(additions_changed_event(agent_id, &view)))
    }

    fn stop_agent(&self, request: &Request) -> Result<Success, HandlerError> {
        let agent_id = parse_agent_id(request)?;
        if let Some(subtree) = self.subtree.get() {
            return subtree_stop(subtree.as_ref(), request, agent_id);
        }
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

/// The session that a start made, and the status event of its agent.
fn started_session(outcome: StartOutcome) -> Success {
    let status = outcome.agent.status;
    let detail = outcome.failure.as_deref();
    let event = agent_status_event(outcome.agent.id, status, detail);
    let view = fresh_session_view(outcome.session);
    Success::result(session_json(&view)).with_event(event)
}

fn parse_agent_id(request: &Request) -> Result<AgentId, HandlerError> {
    let body = AgentIdRequest::read(&request.fields).map_err(HandlerError::invalid)?;
    parse_id("agent_id", &body.agent_id)
}

/// Stops one agent or, when the request asks for it, its whole subtree
/// (FR-007). The events are one per stopped agent.
fn subtree_stop(
    subtree: &dyn SubtreeRequests,
    request: &Request,
    agent_id: AgentId,
) -> Result<Success, HandlerError> {
    subtree_outcome(subtree.stop_subtree(request, agent_id), |agent_id| {
        agent_status_event(agent_id, AgentStatus::Stopped, None)
    })
}

/// Deletes one agent or, when the request confirms the fresh descendant
/// count, its whole subtree (FR-006). The events are one per deleted agent.
fn subtree_delete(
    subtree: &dyn SubtreeRequests,
    request: &Request,
    agent_id: AgentId,
) -> Result<Success, HandlerError> {
    subtree_outcome(
        subtree.delete_subtree(request, agent_id),
        agent_deleted_event,
    )
}

/// The answer to a subtree action: a null result with one event per
/// affected agent, or the wire failure the bridge reported.
fn subtree_outcome(
    result: SubtreeResult,
    event_of: impl Fn(AgentId) -> Event,
) -> Result<Success, HandlerError> {
    match result {
        SubtreeResult::Failed { code, message } => Err(HandlerError::with_code(code, message)),
        SubtreeResult::Stopped(ids) | SubtreeResult::Deleted(ids) => {
            let events = ids.into_iter().map(event_of).collect();
            Ok(Success {
                result: Value::Null,
                events,
            })
        }
    }
}

fn parse_id<T: std::str::FromStr>(field: &str, text: &str) -> Result<T, HandlerError> {
    text.parse()
        .map_err(|_| HandlerError::invalid(format!("{field} is not a valid identifier: {text}")))
}

#[cfg(test)]
#[path = "agent_handlers.tests.rs"]
mod tests;
