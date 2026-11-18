// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use notification_presentation::NotificationHandlers;
use tokio::task::spawn_blocking;
use wire::{ErrorCode, Request, Response};

use super::event_hub::EventHub;

/// Runs the `notification.*` requests. The use cases block, so each call runs
/// on a blocking thread.
#[derive(Clone)]
pub struct NotificationGateway {
    handlers: Arc<NotificationHandlers>,
    /// The gateway publishes nothing itself: no request of this module
    /// causes an event (D5 keeps `notification.created` to the bridge).
    _events: EventHub,
}

impl NotificationGateway {
    #[must_use]
    pub fn new(handlers: Arc<NotificationHandlers>, events: EventHub) -> Self {
        Self {
            handlers,
            _events: events,
        }
    }

    /// True for the requests that this gateway answers: the whole of
    /// `notification.`.
    #[must_use]
    pub fn accepts(kind: &str) -> bool {
        kind.starts_with(notification_presentation::REQUEST_PREFIX)
    }

    pub async fn handle(&self, request: Request) -> Response {
        let id = request.id;
        let handlers = Arc::clone(&self.handlers);
        spawn_blocking(move || handlers.handle(&request))
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
