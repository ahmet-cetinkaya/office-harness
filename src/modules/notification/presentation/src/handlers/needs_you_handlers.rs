// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::AgentId;
use notification_application::common::errors::NotificationApplicationError;
use notification_domain::NeedsYouItem;
use serde_json::{Value, json};
use wire::{Request, Response};

use super::handler_error::HandlerError;
use super::notification_handlers::parse_id;
use crate::contracts::{DismissNeedsYouRequest, ListNeedsYouRequest};
use crate::mappings::needs_you_item_json;

/// The request types of the Needs you queue all start with this prefix.
pub const NEEDS_YOU_REQUEST_PREFIX: &str = "needs_you.";

const NO_ITEM_MESSAGE: &str = "no needs-you item for the agent";

/// What the handlers need from the queue. The host implements it so that a
/// dismiss also publishes `needs_you.changed` (0008 contract).
pub trait NeedsYouUseCases: Send + Sync {
    /// # Errors
    /// `Storage` when the items cannot be read.
    fn list(&self) -> Result<Vec<NeedsYouItem>, NotificationApplicationError>;

    /// Removes the agent's item and returns the list that is left.
    ///
    /// # Errors
    /// `NotFound` when the agent has no item, `Storage`.
    fn dismiss(&self, agent_id: AgentId)
    -> Result<Vec<NeedsYouItem>, NotificationApplicationError>;
}

/// The handlers of `needs_you.list` and `needs_you.dismiss`. They block, so
/// the host runs them on a blocking thread.
#[derive(Clone)]
pub struct NeedsYouHandlers {
    use_cases: Arc<dyn NeedsYouUseCases>,
}

impl NeedsYouHandlers {
    #[must_use]
    pub fn new(use_cases: Arc<dyn NeedsYouUseCases>) -> Self {
        Self { use_cases }
    }

    /// True for the requests that these handlers answer.
    #[must_use]
    pub fn accepts(kind: &str) -> bool {
        kind.starts_with(NEEDS_YOU_REQUEST_PREFIX)
    }

    #[must_use]
    pub fn handle(&self, request: &Request) -> Response {
        let id = request.id;
        match self.route(request) {
            Ok(items) => Response::success(id, items_result(&items)),
            Err(error) => Response::failure(id, error.code, error.message),
        }
    }

    fn route(&self, request: &Request) -> Result<Vec<NeedsYouItem>, HandlerError> {
        match request.kind.as_str() {
            "needs_you.list" => self.list(request),
            "needs_you.dismiss" => self.dismiss(request),
            other => Err(HandlerError::not_found(format!(
                "no request of the type {other}"
            ))),
        }
    }

    fn list(&self, request: &Request) -> Result<Vec<NeedsYouItem>, HandlerError> {
        ListNeedsYouRequest::read(&request.fields).map_err(HandlerError::invalid)?;
        Ok(self.use_cases.list()?)
    }

    fn dismiss(&self, request: &Request) -> Result<Vec<NeedsYouItem>, HandlerError> {
        let body = DismissNeedsYouRequest::read(&request.fields).map_err(HandlerError::invalid)?;
        let agent_id = parse_id::<AgentId>("agent_id", &body.agent_id)?;
        self.use_cases
            .dismiss(agent_id)
            .map_err(|error| match error {
                NotificationApplicationError::NotFound(_) => {
                    HandlerError::not_found(NO_ITEM_MESSAGE)
                }
                other @ NotificationApplicationError::Storage { .. } => other.into(),
            })
    }
}

fn items_result(items: &[NeedsYouItem]) -> Value {
    json!({ "items": items.iter().map(needs_you_item_json).collect::<Vec<_>>() })
}
