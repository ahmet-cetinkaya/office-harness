// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use hierarchy_presentation::{HierarchyHandlers, REQUEST_PREFIXES};
use tokio::task::spawn_blocking;
use wire::{ErrorCode, Request, Response};

use super::event_hub::EventHub;

/// Runs the `team.*` and `hierarchy.*` requests. The use cases block, so
/// each call runs on a blocking thread. The events that a request causes go
/// to every client.
#[derive(Clone)]
pub struct HierarchyGateway {
    handlers: Arc<HierarchyHandlers>,
    events: EventHub,
}

impl HierarchyGateway {
    #[must_use]
    pub fn new(handlers: Arc<HierarchyHandlers>, events: EventHub) -> Self {
        Self { handlers, events }
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

    fn run(&self, request: &Request) -> Response {
        let handled = self.handlers.handle(request);
        for event in &handled.events {
            self.events.publish(event);
        }
        handled.response
    }
}
