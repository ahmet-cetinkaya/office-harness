// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use std::str::FromStr;

use kernel::AgentId;
use notification_application::commands::needs_you::NeedsYouClearReason;
use notification_domain::NotificationId;
use notification_presentation::{NeedsYouHandlers, NotificationHandlers};
use serde_json::Value;
use tokio::task::spawn_blocking;
use wire::{ErrorCode, Request, Response};

use crate::bridges::NeedsYouBridge;

const READ_REQUEST: &str = "notification.read";
const READ_AGENT_REQUEST: &str = "notification.read_agent";
const NOTIFICATION_ID_FIELD: &str = "notification_id";
const AGENT_ID_FIELD: &str = "agent_id";

/// Runs the `notification.*` and `needs_you.*` requests. The use cases block,
/// so each call runs on a blocking thread. A read that succeeded also clears the Needs you item
/// it answers (0008 D2); the queue bridge publishes that change, so the
/// gateway publishes nothing itself (D5 keeps `notification.created` to the
/// notification bridge).
#[derive(Clone)]
pub struct NotificationGateway {
    handlers: Arc<NotificationHandlers>,
    needs_you_handlers: NeedsYouHandlers,
    needs_you: NeedsYouBridge,
}

impl NotificationGateway {
    #[must_use]
    pub fn new(handlers: Arc<NotificationHandlers>, needs_you: NeedsYouBridge) -> Self {
        Self {
            handlers,
            needs_you_handlers: NeedsYouHandlers::new(Arc::new(needs_you.clone())),
            needs_you,
        }
    }

    /// True for the requests that this gateway answers: the whole of
    /// `notification.` and of `needs_you.`.
    #[must_use]
    pub fn accepts(kind: &str) -> bool {
        NotificationHandlers::accepts(kind) || NeedsYouHandlers::accepts(kind)
    }

    pub async fn handle(&self, request: Request) -> Response {
        let id = request.id;
        let handlers = Arc::clone(&self.handlers);
        let needs_you_handlers = self.needs_you_handlers.clone();
        let needs_you = self.needs_you.clone();
        spawn_blocking(move || {
            if NeedsYouHandlers::accepts(&request.kind) {
                return needs_you_handlers.handle(&request);
            }
            let response = handlers.handle(&request);
            if matches!(response, Response::Success { .. }) {
                clear_answered_item(&needs_you, &request);
            }
            response
        })
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

/// A read of an agent's notifications, or of the notification that created an
/// item, clears the item. The handlers already accepted the request, so a
/// field that does not parse here is simply not acted on.
fn clear_answered_item(needs_you: &NeedsYouBridge, request: &Request) {
    let text_of = |field: &str| request.fields.get(field).and_then(Value::as_str);
    match request.kind.as_str() {
        READ_AGENT_REQUEST => {
            if let Some(agent_id) =
                text_of(AGENT_ID_FIELD).and_then(|text| AgentId::from_str(text).ok())
            {
                needs_you.clear(agent_id, NeedsYouClearReason::TerminalOpened);
            }
        }
        READ_REQUEST => {
            if let Some(id) =
                text_of(NOTIFICATION_ID_FIELD).and_then(|text| NotificationId::from_str(text).ok())
            {
                needs_you.clear_on_read(id);
            }
        }
        _ => {}
    }
}
