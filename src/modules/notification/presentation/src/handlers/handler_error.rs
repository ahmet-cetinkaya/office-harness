// SPDX-License-Identifier: AGPL-3.0-or-later

use notification_application::common::errors::NotificationApplicationError;
use wire::ErrorCode;

/// A failed request: the wire code and a message that names the field or
/// entry at fault.
#[derive(Debug, PartialEq, Eq)]
pub struct HandlerError {
    pub code: ErrorCode,
    pub message: String,
}

impl HandlerError {
    #[must_use]
    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Invalid,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::NotFound,
            message: message.into(),
        }
    }
}

impl From<NotificationApplicationError> for HandlerError {
    fn from(error: NotificationApplicationError) -> Self {
        let message = error.to_string();
        match error {
            NotificationApplicationError::NotFound(_) => Self {
                code: ErrorCode::NotFound,
                message,
            },
            NotificationApplicationError::Storage { .. } => Self {
                code: ErrorCode::Internal,
                message,
            },
        }
    }
}
