// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, PoisonError};

use agent_presentation::{AgentHandlers, Handled, REQUEST_PREFIXES};
use audit_presentation::{ACTIONS_REQUEST, AuditHandlers};
use tokio::task::spawn_blocking;
use wire::{ErrorCode, Request, Response};

use super::event_hub::EventHub;
use crate::bootstrap::StorageAlerts;
use crate::bridges::{StatusOrder, take_recorded_events};

/// The requests that change an agent's status or its terminal. They run one
/// at a time, and apart from the status bridge, so that a delete never
/// overtakes a slow start, and a stop never races an exit.
const ORDERED_REQUESTS: [&str; 3] = ["agent.start", "agent.stop", "agent.delete"];

/// Runs the `agent.*`, `project.*`, and `settings.*` requests. The use cases
/// block, so each call runs on a blocking thread. The events that a request
/// causes go to every client, and a failed disk raises `storage.error`.
#[derive(Clone)]
pub struct AgentGateway {
    agents: Arc<AgentHandlers>,
    audit: AuditHandlers,
    events: EventHub,
    storage_alerts: StorageAlerts,
    status_order: StatusOrder,
}

impl AgentGateway {
    #[must_use]
    pub fn new(
        agents: Arc<AgentHandlers>,
        audit: AuditHandlers,
        events: EventHub,
        storage_alerts: StorageAlerts,
        status_order: StatusOrder,
    ) -> Self {
        Self {
            agents,
            audit,
            events,
            storage_alerts,
            status_order,
        }
    }

    /// True for the requests that this gateway answers: the whole of
    /// `agent.`, `project.`, and `settings.`.
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
}

impl AgentGateway {
    fn run(&self, request: &Request) -> Response {
        if request.kind == ACTIONS_REQUEST {
            let (response, disk_failure) = self.audit.handle(request);
            self.raise(disk_failure.as_deref());
            return response;
        }
        let _turn = ORDERED_REQUESTS.contains(&request.kind.as_str()).then(|| {
            self.status_order
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
        });
        // Drop what an earlier request on this thread left behind.
        drop(take_recorded_events());
        let Handled {
            response,
            events,
            disk_failure,
        } = self.agents.handle(request);
        self.raise(disk_failure.as_deref());
        for event in events.iter().chain(&take_recorded_events()) {
            self.events.publish(event);
        }
        response
    }

    fn raise(&self, disk_failure: Option<&str>) {
        if let Some(message) = disk_failure {
            self.storage_alerts.report_disk_failure(message);
        }
    }
}
