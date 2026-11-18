// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::NotificationError;

/// Identifier of a notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NotificationId(Uuid);

impl NotificationId {
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// Reads the id as stored text. `None` for any text that is not a UUID.
    #[must_use]
    pub fn from_uuid_text(text: &str) -> Option<Self> {
        Uuid::parse_str(text).ok().map(Self)
    }

    #[must_use]
    pub fn as_uuid_text(&self) -> String {
        self.0.to_string()
    }
}

impl fmt::Display for NotificationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for NotificationId {
    type Err = NotificationError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(text)
            .map(Self)
            .map_err(|_| NotificationError(format!("notification_id is not a valid UUID: {text}")))
    }
}
