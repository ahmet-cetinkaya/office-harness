// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::errors::ApplicationError;
use wire::ErrorCode;

/// A failed request: the wire code, a message that names the field or entry,
/// and the disk failure text when the disk is at fault.
#[derive(Debug, PartialEq, Eq)]
pub struct HandlerError {
    pub code: ErrorCode,
    pub message: String,
    pub disk_failure: Option<String>,
}

impl HandlerError {
    #[must_use]
    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Invalid,
            message: message.into(),
            disk_failure: None,
        }
    }

    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::NotFound,
            message: message.into(),
            disk_failure: None,
        }
    }
}

impl From<ApplicationError> for HandlerError {
    fn from(error: ApplicationError) -> Self {
        let message = error.to_string();
        let (code, disk_failure) = match error {
            ApplicationError::Invalid(_) => (ErrorCode::Invalid, None),
            ApplicationError::NotFound(_) => (ErrorCode::NotFound, None),
            ApplicationError::Conflict(_) => (ErrorCode::Conflict, None),
            ApplicationError::Refused(_) => (ErrorCode::Refused, None),
            ApplicationError::Internal(_) => (ErrorCode::Internal, None),
            ApplicationError::Storage {
                is_disk_failure, ..
            } => (
                ErrorCode::Internal,
                is_disk_failure.then(|| message.clone()),
            ),
        };
        Self {
            code,
            message,
            disk_failure,
        }
    }
}
