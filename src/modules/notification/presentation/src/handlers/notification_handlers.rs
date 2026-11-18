// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;
use notification_application::commands::read_agent_notifications::ReadAgentNotificationsHandler;
use notification_application::commands::read_notification::ReadNotificationHandler;
use notification_application::queries::list_notifications::ListNotificationsHandler;
use notification_domain::NotificationId;
use serde_json::Value;
use wire::{Request, Response};

use super::handler_error::HandlerError;
use crate::contracts::{ListNotificationsRequest, NotificationIdRequest, ReadAgentRequest};
use crate::mappings::notification_json;

/// The request types of this module all start with this prefix.
pub const REQUEST_PREFIX: &str = "notification.";

/// The use cases that the handlers call.
#[derive(Clone)]
pub struct NotificationUseCases {
    pub list: ListNotificationsHandler,
    pub read: ReadNotificationHandler,
    pub read_agent: ReadAgentNotificationsHandler,
}

/// The handlers of the notification requests. They block, so the host runs
/// them on a blocking thread. No request of this module publishes an event:
/// a read changes no visible state for other clients (D5), and
/// `notification.created` is the bridge's event, not a request's.
#[derive(Clone)]
pub struct NotificationHandlers {
    use_cases: NotificationUseCases,
}

impl NotificationHandlers {
    #[must_use]
    pub fn new(use_cases: NotificationUseCases) -> Self {
        Self { use_cases }
    }

    /// True for the requests that these handlers answer.
    #[must_use]
    pub fn accepts(kind: &str) -> bool {
        kind.starts_with(REQUEST_PREFIX)
    }

    #[must_use]
    pub fn handle(&self, request: &Request) -> Response {
        let id = request.id;
        match self.route(request) {
            Ok(result) => Response::success(id, result),
            Err(error) => Response::failure(id, error.code, error.message),
        }
    }

    fn route(&self, request: &Request) -> Result<Value, HandlerError> {
        match request.kind.as_str() {
            "notification.list" => self.list(request),
            "notification.read" => self.read(request),
            "notification.read_agent" => self.read_agent(request),
            other => Err(HandlerError::not_found(format!(
                "no request of the type {other}"
            ))),
        }
    }

    fn list(&self, request: &Request) -> Result<Value, HandlerError> {
        let body =
            ListNotificationsRequest::read(&request.fields).map_err(HandlerError::invalid)?;
        let notifications = if body.unread_only {
            self.use_cases.list.handle_unread()?
        } else {
            self.use_cases.list.handle()?
        };
        Ok(notifications.iter().map(notification_json).collect())
    }

    fn read(&self, request: &Request) -> Result<Value, HandlerError> {
        let body = NotificationIdRequest::read(&request.fields).map_err(HandlerError::invalid)?;
        let notification_id: NotificationId = parse_id("notification_id", &body.notification_id)?;
        self.use_cases.read.handle(notification_id)?;
        Ok(Value::Null)
    }

    fn read_agent(&self, request: &Request) -> Result<Value, HandlerError> {
        let body = ReadAgentRequest::read(&request.fields).map_err(HandlerError::invalid)?;
        let agent_id = parse_id::<AgentId>("agent_id", &body.agent_id)?;
        self.use_cases.read_agent.handle(agent_id)?;
        Ok(Value::Null)
    }
}

fn parse_id<T: std::str::FromStr>(field: &str, text: &str) -> Result<T, HandlerError> {
    text.parse()
        .map_err(|_| HandlerError::invalid(format!("{field} is not a valid identifier: {text}")))
}

#[cfg(test)]
#[path = "notification_handlers.tests.rs"]
mod tests;
