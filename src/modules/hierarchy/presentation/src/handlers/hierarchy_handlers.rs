// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use hierarchy_application::commands::subtree::SubtreeRequest;
use hierarchy_application::common::errors::HierarchyApplicationError;
use hierarchy_application::queries::tree::TreeAgent;
use hierarchy_application::{
    CreateTeamHandler, DeleteTeamHandler, PlaceAgentHandler, SubtreeHandler, TreeQuery,
};
use kernel::AgentId;
use serde_json::{Value, json};
use wire::{Event, Request, Response};

/// The request prefixes this module answers.
pub const REQUEST_PREFIXES: [&str; 2] = ["team.", "hierarchy."];

/// The tree event every client gets after a change (D6).
pub const TREE_EVENT: &str = "hierarchy.changed";

/// The use cases the hierarchy handlers call.
pub struct HierarchyUseCases {
    pub tree: Arc<TreeQuery>,
    pub create_team: CreateTeamHandler,
    pub delete_team: DeleteTeamHandler,
    pub place_agent: PlaceAgentHandler,
    pub subtree: SubtreeHandler,
}

/// The answer to one request, with the events that every client gets.
#[derive(Debug)]
pub struct Handled {
    pub response: Response,
    pub events: Vec<Event>,
}

/// The handlers of the team and hierarchy requests. They block, so the host
/// runs them on a blocking thread.
pub struct HierarchyHandlers {
    use_cases: HierarchyUseCases,
}

impl HierarchyHandlers {
    #[must_use]
    pub fn new(use_cases: HierarchyUseCases) -> Self {
        Self { use_cases }
    }

    #[must_use]
    pub fn handle(&self, request: &Request) -> Handled {
        let id = request.id;
        match self.route(request) {
            Ok(success) => Handled {
                response: Response::success(id, success.result),
                events: success.events,
            },
            Err(error) => Handled {
                response: Response::failure(id, error.code, error.message),
                events: Vec::new(),
            },
        }
    }

    fn route(&self, request: &Request) -> Result<Success, HandlerError> {
        match request.kind.as_str() {
            "hierarchy.tree" => self.tree(),
            "team.create" => self.create_team(request),
            "team.delete" => self.delete_team(request),
            "hierarchy.descendants" => self.descendants(request),
            other => Err(HandlerError::not_found(format!(
                "no request of the type {other}"
            ))),
        }
    }

    fn tree(&self) -> Result<Success, HandlerError> {
        let tree = self
            .use_cases
            .tree
            .fetch()
            .map_err(|error| handler_error(&error))?;
        Ok(Success::result(tree_json(&tree)))
    }

    fn create_team(&self, request: &Request) -> Result<Success, HandlerError> {
        let name = read_string(&request.fields, "name").ok_or_else(missing("name"))?;
        let project_id = read_id(&request.fields, "project_id");
        let lead_agent_id =
            read_agent_id(&request.fields, "lead_agent_id").ok_or_else(missing("lead_agent_id"))?;
        let team = self
            .use_cases
            .create_team
            .handle(
                &hierarchy_application::commands::create_team::CreateTeamRequest {
                    name,
                    project_id,
                    lead_agent_id,
                },
            )
            .map_err(|error| handler_error(&error))?;
        Ok(Success::result(team_json(&team)).with_event(tree_event("team.created")))
    }

    fn delete_team(&self, request: &Request) -> Result<Success, HandlerError> {
        let team_id = read_team_id(&request.fields, "team_id").ok_or_else(missing("team_id"))?;
        let confirm = request
            .fields
            .get("confirm")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        self.use_cases
            .delete_team
            .handle(
                hierarchy_application::commands::delete_team::DeleteTeamRequest {
                    team_id,
                    confirm,
                },
            )
            .map_err(|error| handler_error(&error))?;
        Ok(Success::done().with_event(tree_event("team.deleted")))
    }

    fn descendants(&self, request: &Request) -> Result<Success, HandlerError> {
        let agent_id =
            read_agent_id(&request.fields, "agent_id").ok_or_else(missing("agent_id"))?;
        let count = self
            .use_cases
            .subtree
            .descendant_count(agent_id)
            .map_err(|error| handler_error(&error))?;
        Ok(Success::result(json!({ "count": count })))
    }
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

/// The subtree stop and delete need the agent module's `agent.stop` and
/// `agent.delete` shapes, so they extend those requests (FR-006, FR-007).
/// The extension lives in the agent handlers; this builds the shared
/// request model from the wire fields.
#[must_use]
pub fn subtree_request(request: &Request, agent_id: AgentId) -> SubtreeRequest {
    SubtreeRequest {
        agent_id,
        include_descendants: request
            .fields
            .get("include_descendants")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        confirmed_descendants: request
            .fields
            .get("descendant_count")
            .and_then(Value::as_u64)
            .and_then(|count| usize::try_from(count).ok()),
    }
}

fn tree_event(cause: &str) -> Event {
    Event::new(TREE_EVENT, {
        let mut fields = serde_json::Map::new();
        fields.insert("cause".to_owned(), Value::String(cause.to_owned()));
        fields
    })
}

fn tree_json(tree: &hierarchy_application::queries::tree::Tree) -> Value {
    json!({
        "ceo": tree.ceo.as_ref().map(agent_json),
        "teams": tree.teams.iter().map(|team| json!({
            "id": team.team.id().as_uuid_text(),
            "name": team.team.name().display(),
            "project_id": team.team.project_id().map(|id| id.as_uuid_text()),
            "lead": agent_json(&team.lead),
            "members": team.members.iter().map(agent_json).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "unassigned": tree.unassigned.iter().map(agent_json).collect::<Vec<_>>(),
    })
}

fn agent_json(agent: &TreeAgent) -> Value {
    json!({
        "id": agent.agent_id.as_uuid_text(),
        "role": agent.role.map(hierarchy_domain::Role::as_str),
        "team_id": agent.team_id.map(|team| team.as_uuid_text()),
    })
}

fn team_json(team: &hierarchy_domain::Team) -> Value {
    json!({
        "id": team.id().as_uuid_text(),
        "name": team.name().display(),
        "project_id": team.project_id().map(|id| id.as_uuid_text()),
        "lead_agent_id": team.lead_agent_id().as_uuid_text(),
    })
}

fn read_string(fields: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    fields.get(key)?.as_str().map(str::to_owned)
}

fn read_agent_id(fields: &serde_json::Map<String, Value>, key: &str) -> Option<AgentId> {
    AgentId::from_uuid_text(fields.get(key)?.as_str()?)
}

fn read_team_id(
    fields: &serde_json::Map<String, Value>,
    key: &str,
) -> Option<hierarchy_domain::TeamId> {
    hierarchy_domain::TeamId::from_uuid_text(fields.get(key)?.as_str()?)
}

fn read_id(fields: &serde_json::Map<String, Value>, key: &str) -> Option<kernel::ProjectId> {
    kernel::ProjectId::from_uuid_text(fields.get(key)?.as_str()?)
}

fn missing(field: &'static str) -> impl Fn() -> HandlerError {
    move || HandlerError::invalid(format!("{field} is missing"))
}

struct HandlerError {
    code: wire::ErrorCode,
    message: String,
}

impl HandlerError {
    fn not_found(message: String) -> Self {
        Self {
            code: wire::ErrorCode::NotFound,
            message,
        }
    }

    fn invalid(message: String) -> Self {
        Self {
            code: wire::ErrorCode::Invalid,
            message,
        }
    }
}

fn handler_error(error: &HierarchyApplicationError) -> HandlerError {
    let code = match error {
        HierarchyApplicationError::Invalid(_) | HierarchyApplicationError::InvalidInput(_) => {
            wire::ErrorCode::Invalid
        }
        HierarchyApplicationError::NotFound(_) => wire::ErrorCode::NotFound,
        HierarchyApplicationError::Conflict(_) => wire::ErrorCode::Conflict,
        HierarchyApplicationError::Storage(_) => wire::ErrorCode::Internal,
    };
    HandlerError {
        code,
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A creator refusal rides to the wire as `invalid` with the agent
    /// module's own text (#33 r2 LOW: the mapping is small, but the code a
    /// client sees is the contract, so it is asserted).
    #[test]
    fn an_invalid_input_maps_to_the_wire_invalid_code() {
        let error =
            HierarchyApplicationError::InvalidInput("agent name must not be empty".to_owned());
        let mapped = handler_error(&error);
        assert_eq!(mapped.code, wire::ErrorCode::Invalid);
        assert_eq!(mapped.message, "agent name must not be empty");
    }

    #[test]
    fn a_not_found_maps_to_the_wire_not_found_code() {
        let error = HierarchyApplicationError::NotFound("the team is gone".to_owned());
        let mapped = handler_error(&error);
        assert_eq!(mapped.code, wire::ErrorCode::NotFound);
        assert_eq!(mapped.message, "not found: the team is gone");
    }
}
