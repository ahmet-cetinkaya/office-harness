// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Map, Value};

use super::Fields;

/// `notification.list`.
#[derive(Debug)]
pub struct ListNotificationsRequest {
    pub unread_only: bool,
}

impl ListNotificationsRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self {
            unread_only: Fields(body).optional_bool("unread_only")?.unwrap_or(false),
        })
    }
}

/// `notification.read`.
#[derive(Debug)]
pub struct NotificationIdRequest {
    pub notification_id: String,
}

impl NotificationIdRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self {
            notification_id: Fields(body).required_text("notification_id")?,
        })
    }
}

/// `notification.read_agent`.
#[derive(Debug)]
pub struct ReadAgentRequest {
    pub agent_id: String,
}

impl ReadAgentRequest {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self {
            agent_id: Fields(body).required_text("agent_id")?,
        })
    }
}
