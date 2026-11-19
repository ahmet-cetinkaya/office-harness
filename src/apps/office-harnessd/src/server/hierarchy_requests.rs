// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_application::queries::list_agents::ListAgentsHandler;
use agent_presentation::mappings::agent_changed_event;
use hierarchy_presentation::{HierarchyHandlers, REQUEST_PREFIXES};
use kernel::AgentId;
use serde_json::Value;
use tokio::task::spawn_blocking;
use wire::{ErrorCode, Event, Request, Response};

use super::event_hub::EventHub;
use crate::bootstrap::StorageAlerts;
use crate::bridges::take_recorded_events;

/// The request whose `lead` object creates an agent (0002 contract).
const TEAM_CREATE_REQUEST: &str = "team.create";

/// Runs the `team.*` and `hierarchy.*` requests. The use cases block, so
/// each call runs on a blocking thread. The events that a request causes go
/// to every client.
#[derive(Clone)]
pub struct HierarchyGateway {
    handlers: Arc<HierarchyHandlers>,
    agents: ListAgentsHandler,
    events: EventHub,
    storage_alerts: StorageAlerts,
}

impl HierarchyGateway {
    #[must_use]
    pub fn new(
        handlers: Arc<HierarchyHandlers>,
        agents: ListAgentsHandler,
        events: EventHub,
        storage_alerts: StorageAlerts,
    ) -> Self {
        Self {
            handlers,
            agents,
            events,
            storage_alerts,
        }
    }

    /// True for the requests that this gateway answers: the whole of
    /// `team.` and `hierarchy.`.
    #[must_use]
    pub fn accepts(kind: &str) -> bool {
        REQUEST_PREFIXES
            .iter()
            .any(|prefix| kind.starts_with(prefix))
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

    /// Publishes the new lead first, so a client knows the agent before the
    /// tree names it, and the action records last, after the change itself.
    fn run(&self, request: &Request) -> Response {
        // Drop what an earlier request on this thread left behind.
        drop(take_recorded_events());
        let handled = self.handlers.handle(request);
        let new_lead = self.new_lead_event(request, &handled.response);
        for event in new_lead
            .iter()
            .chain(&handled.events)
            .chain(&take_recorded_events())
        {
            self.events.publish(event);
        }
        handled.response
    }

    /// `agent.changed` for the lead that a successful `team.create` with a
    /// `lead` object made: a client adds agents to its list only on that
    /// event. A failed request made no agent, so it has none.
    fn new_lead_event(&self, request: &Request, response: &Response) -> Option<Event> {
        if request.kind != TEAM_CREATE_REQUEST || !request.fields.contains_key("lead") {
            return None;
        }
        let Response::Success { result, .. } = response else {
            return None;
        };
        let lead_agent_id = result
            .get("lead_agent_id")
            .and_then(Value::as_str)
            .and_then(AgentId::from_uuid_text)?;
        match self.agents.handle_one(lead_agent_id) {
            Ok(lead) => Some(agent_changed_event(&lead)),
            Err(error) => {
                // The team and its lead are saved; the read back failed, so
                // the clients learn of the disk fault instead of the agent.
                self.storage_alerts.report_disk_failure(&error.to_string());
                None
            }
        }
    }
}
