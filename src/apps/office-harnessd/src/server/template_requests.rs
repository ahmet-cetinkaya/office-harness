// SPDX-License-Identifier: AGPL-3.0-or-later

use std::str::FromStr;
use std::sync::Arc;

use agent_presentation::mappings::{agent_changed_event, agent_json};
use hierarchy_domain::{Team, TeamId};
use hierarchy_presentation::{CAUSE_AGENT_PLACED, CAUSE_TEAM_CREATED, tree_event};
use kernel::{AgentId, ProjectId};
use serde_json::{Value, json};
use template_domain::TemplateId;
use template_presentation::{
    CREATE_AGENT, CREATE_TEAM, CreateAgentFields, CreateTeamFields, PlaceFields, REQUEST_PREFIX,
    TemplateHandlers, refuse_unknown_fields,
};
use tokio::task::spawn_blocking;
use wire::{ErrorCode, Event, Request, Response};

use super::event_hub::EventHub;
use crate::bootstrap::StorageAlerts;
use crate::bridges::{
    CreatedAgent, DirectPlace, TeamCreation, TemplateCreateBridge, TemplateCreateError,
    take_recorded_events,
};

/// The answer to one request and the events that every client gets.
struct Answered {
    response: Response,
    events: Vec<Event>,
}

/// Runs the `template.*` requests, for the user's connections only: the
/// agent connection answers none of them. The module's own requests run in
/// its handlers; the two create requests cross modules, so the host's create
/// bridge answers them. The use cases block, so each call runs on a blocking
/// thread, and every change goes to all clients (0009 D7).
#[derive(Clone)]
pub struct TemplateGateway {
    handlers: Arc<TemplateHandlers>,
    create: Arc<TemplateCreateBridge>,
    events: EventHub,
    alerts: StorageAlerts,
}

impl TemplateGateway {
    #[must_use]
    pub fn new(
        handlers: Arc<TemplateHandlers>,
        create: Arc<TemplateCreateBridge>,
        events: EventHub,
        alerts: StorageAlerts,
    ) -> Self {
        Self {
            handlers,
            create,
            events,
            alerts,
        }
    }

    /// True for the requests that this gateway answers: the whole of
    /// `template.`.
    #[must_use]
    pub fn accepts(kind: &str) -> bool {
        kind.starts_with(REQUEST_PREFIX)
    }

    pub async fn handle(&self, request: Request) -> Response {
        let id = request.id;
        let gateway = self.clone();
        spawn_blocking(move || gateway.run(&request))
            .await
            .unwrap_or_else(|error| {
                Response::failure(
                    id,
                    ErrorCode::Internal,
                    format!("the request task failed: {error}"),
                )
            })
    }

    /// The answer of one request on the blocking pool. The events of the
    /// change go first, and the action records that the creates wrote go
    /// last, so a client sees the change before its record. A rolled-back
    /// creation still publishes its records: the Create and Delete entries
    /// exist.
    fn run(&self, request: &Request) -> Response {
        if let Err(message) = refuse_unknown_fields(request) {
            return Response::failure(request.id, ErrorCode::Invalid, message);
        }
        // Drop what an earlier request on this thread left behind.
        drop(take_recorded_events());
        let Answered { response, events } = match request.kind.as_str() {
            CREATE_AGENT => self.create_agent(request),
            CREATE_TEAM => self.create_team(request),
            _ => self.handle_in_module(request),
        };
        for event in events.iter().chain(&take_recorded_events()) {
            self.events.publish(event);
        }
        response
    }

    fn handle_in_module(&self, request: &Request) -> Answered {
        let handled = self.handlers.handle(request);
        if let Some(message) = &handled.disk_failure {
            self.alerts.report_disk_failure(message);
        }
        Answered {
            response: handled.response,
            events: handled.events,
        }
    }

    fn create_agent(&self, request: &Request) -> Answered {
        match self.try_create_agent(request) {
            Ok((result, events)) => answered(request, Ok(result), events),
            Err(error) => answered(request, Err(error), Vec::new()),
        }
    }

    fn try_create_agent(
        &self,
        request: &Request,
    ) -> Result<(Value, Vec<Event>), TemplateCreateError> {
        let fields = CreateAgentFields::read(&request.fields).map_err(field_failure)?;
        let template_id = parse_template_id("template_id", &fields.template_id)?;
        let place = fields.place.as_ref().map(direct_place).transpose()?;
        let project_id = fields
            .project_id
            .as_deref()
            .map(parse_project_id)
            .transpose()?;
        let created = self.create.create_agent(template_id, place, project_id)?;
        Ok((
            json!({"agent": created_agent_json(&created)}),
            created_agent_events(&created),
        ))
    }

    fn create_team(&self, request: &Request) -> Answered {
        match self.try_create_team(request) {
            Ok((result, events)) => answered(request, Ok(result), events),
            Err(error) => answered(request, Err(error), Vec::new()),
        }
    }

    fn try_create_team(
        &self,
        request: &Request,
    ) -> Result<(Value, Vec<Event>), TemplateCreateError> {
        let fields = CreateTeamFields::read(&request.fields).map_err(field_failure)?;
        let template_id = parse_template_id("template_id", &fields.template_id)?;
        let parent_agent_id = parse_agent_id("parent_agent_id", &fields.parent_agent_id)?;
        let project_id = fields
            .project_id
            .as_deref()
            .map(parse_project_id)
            .transpose()?;
        let created =
            self.create
                .create_team(template_id, &fields.team_name, parent_agent_id, project_id)?;
        Ok((team_creation_json(&created), team_events(&created)))
    }
}

fn answered(
    request: &Request,
    outcome: Result<Value, TemplateCreateError>,
    events: Vec<Event>,
) -> Answered {
    let response = match outcome {
        Ok(result) => Response::success(request.id, result),
        Err(error) => Response::failure(request.id, error.code, error.message),
    };
    Answered { response, events }
}

fn field_failure(message: String) -> TemplateCreateError {
    TemplateCreateError::invalid(&[message])
}

fn invalid_identifier(field: &str, text: &str) -> TemplateCreateError {
    field_failure(format!("{field} is not a valid identifier: {text}"))
}

fn parse_template_id(field: &str, text: &str) -> Result<TemplateId, TemplateCreateError> {
    TemplateId::from_str(text).map_err(|_| invalid_identifier(field, text))
}

fn parse_agent_id(field: &str, text: &str) -> Result<AgentId, TemplateCreateError> {
    AgentId::from_uuid_text(text).ok_or_else(|| invalid_identifier(field, text))
}

fn parse_project_id(text: &str) -> Result<ProjectId, TemplateCreateError> {
    ProjectId::from_uuid_text(text).ok_or_else(|| invalid_identifier("project_id", text))
}

fn direct_place(place: &PlaceFields) -> Result<DirectPlace, TemplateCreateError> {
    Ok(DirectPlace {
        parent_agent_id: parse_agent_id("place.parent_agent_id", &place.parent_agent_id)?,
        team_id: TeamId::from_uuid_text(&place.team_id)
            .ok_or_else(|| invalid_identifier("place.team_id", &place.team_id))?,
    })
}

/// The agent as `agent.create` answers it, with the place it was given.
fn created_agent_json(created: &CreatedAgent) -> Value {
    let mut agent = agent_json(&created.agent);
    agent["role"] = created
        .role
        .map_or(Value::Null, |role| Value::from(role.as_str()));
    agent["team"] = created.team_name.clone().map_or(Value::Null, Value::from);
    agent
}

fn team_json(team: &Team) -> Value {
    json!({
        "id": team.id().as_uuid_text(),
        "name": team.name().display(),
        "project_id": team.project_id().map(|id| id.as_uuid_text()),
        "lead_agent_id": team.lead_agent_id().as_uuid_text(),
    })
}

fn team_creation_json(created: &TeamCreation) -> Value {
    json!({
        "team": team_json(&created.team),
        "agents": created.agents.iter().map(created_agent_json).collect::<Vec<_>>(),
    })
}

/// The agent first, so a client knows it before the tree names it; the tree
/// changes only when the agent got a place.
fn created_agent_events(created: &CreatedAgent) -> Vec<Event> {
    let mut events = vec![agent_changed_event(&created.agent)];
    if created.role.is_some() {
        events.push(tree_event(CAUSE_AGENT_PLACED));
    }
    events
}

fn team_events(created: &TeamCreation) -> Vec<Event> {
    let mut events: Vec<Event> = created
        .agents
        .iter()
        .map(|created| agent_changed_event(&created.agent))
        .collect();
    events.push(tree_event(CAUSE_TEAM_CREATED));
    events
}
