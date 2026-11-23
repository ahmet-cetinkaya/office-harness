// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use hierarchy_application::commands::hold::HoldService;
use hierarchy_application::queries::tree::TreeQuery;
use hierarchy_domain::AgentAction;
use kernel::AgentId;
use secret_application::SecretService;
use secret_presentation::{
    Handled, USER_REQUESTS, answer_available, answer_get, answer_user_request, secret_changed_event,
};
use tokio::task::spawn_blocking;
use wire::{ErrorCode, Request, Response};

use super::event_hub::EventHub;
use crate::bootstrap::StorageAlerts;
use crate::bridges::SecretJournal;

/// Runs the secret requests: the person's `secret.*` requests on the user
/// connection, and an agent's `secret.available` and `secret.get` on the
/// agent connection (0026). Every request that changes a secret or reads one
/// goes into the action record and, on success, to the clients as
/// `secret.changed`. No record, event, or log line holds a value.
#[derive(Clone)]
pub struct SecretGateway {
    service: Arc<SecretService>,
    journal: SecretJournal,
    events: EventHub,
    holds: Arc<HoldService>,
    tree: Arc<TreeQuery>,
    alerts: StorageAlerts,
}

impl SecretGateway {
    #[must_use]
    pub fn new(
        service: Arc<SecretService>,
        journal: SecretJournal,
        events: EventHub,
        guards: (Arc<HoldService>, Arc<TreeQuery>),
        alerts: StorageAlerts,
    ) -> Self {
        let (holds, tree) = guards;
        Self {
            service,
            journal,
            events,
            holds,
            tree,
            alerts,
        }
    }

    #[must_use]
    pub fn accepts(kind: &str) -> bool {
        USER_REQUESTS.contains(&kind)
    }

    /// A user request. The use cases block, so it runs on a blocking thread.
    pub async fn handle(&self, request: Request) -> Response {
        let id = request.id;
        let gateway = self.clone();
        spawn_blocking(move || {
            let handled = answer_user_request(&gateway.service, request);
            gateway.finish(handled)
        })
        .await
        .unwrap_or_else(|error| task_failed(id, &error))
    }

    /// `secret.available` of an agent: the names and versions it may read. A
    /// hold refuses it; it counts toward no limit and writes no entry.
    pub async fn agent_available(&self, caller: AgentId, request: Request) -> Response {
        let id = request.id;
        let gateway = self.clone();
        spawn_blocking(move || gateway.available_blocking(caller, &request))
            .await
            .unwrap_or_else(|error| task_failed(id, &error))
    }

    /// `secret.get` of an agent. The hold and the reads limit come first, so
    /// a name that does not exist counts toward the limit too (contract).
    pub async fn agent_get(&self, caller: AgentId, request: Request) -> Response {
        let id = request.id;
        let gateway = self.clone();
        spawn_blocking(move || gateway.get_blocking(caller, &request))
            .await
            .unwrap_or_else(|error| task_failed(id, &error))
    }

    /// Drops the grants of agents that were deleted (FR-009). The user's
    /// `agent.delete` calls this after it ran, and the start calls it once.
    pub fn forget_deleted_agents(&self) {
        match self.service.forget_deleted_agents() {
            Ok(true) => self.publish_store_change("revoked"),
            Ok(false) => {}
            Err(error) => self.alerts.report_disk_failure(&error.to_string()),
        }
    }

    fn available_blocking(&self, caller: AgentId, request: &Request) -> Response {
        let rows = match self.tree.store_snapshot() {
            Ok(rows) => rows,
            Err(error) => {
                return Response::failure(request.id, ErrorCode::Internal, error.to_string());
            }
        };
        if let Some(reason) = self.holds.refusal(caller, caller, &rows) {
            return Response::failure(request.id, ErrorCode::Refused, reason);
        }
        answer_available(&self.service, caller, request)
    }

    fn get_blocking(&self, caller: AgentId, request: &Request) -> Response {
        let rows = match self.tree.store_snapshot() {
            Ok(rows) => rows,
            Err(error) => {
                let reason = error.to_string();
                self.journal.read_refused(caller, &reason);
                return Response::failure(request.id, ErrorCode::Internal, reason);
            }
        };
        if let Err(refusal) = self
            .holds
            .guard(caller, AgentAction::SecretRead, None, &rows)
        {
            self.journal
                .read_refused(caller, &format!("refused, {}", refusal.reason));
            return Response::failure(request.id, ErrorCode::Refused, refusal.reason);
        }
        self.finish(answer_get(&self.service, caller, request))
    }

    /// Records what the request said to record, tells the clients, and
    /// returns the answer.
    fn finish(&self, handled: Handled) -> Response {
        if let Some(entry) = &handled.entry {
            self.journal.write(entry);
        }
        if let Some(change) = &handled.event {
            self.events.publish(&secret_changed_event(change));
        }
        handled.response
    }

    fn publish_store_change(&self, cause: &'static str) {
        let change = secret_presentation::ChangeEvent {
            cause,
            name: None,
            agent_id: None,
        };
        self.events.publish(&secret_changed_event(&change));
    }
}

fn task_failed(id: u32, error: &tokio::task::JoinError) -> Response {
    Response::failure(
        id,
        ErrorCode::Internal,
        format!("the request task failed: {error}"),
    )
}
