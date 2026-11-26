// SPDX-License-Identifier: AGPL-3.0-or-later

use schedule_application::common::errors::ScheduleApplicationError;
use schedule_domain::FieldError;
use serde_json::Value;
use wire::ErrorCode;

use crate::mappings::field_errors_json;

/// A failed request: the wire code and a message that names the field or
/// entry at fault. `fields` is the `fields` array of an `invalid` answer
/// (field and rule of each failure). `disk_failure` is the text for
/// `storage.error` when the disk, not the request, is at fault.
#[derive(Debug, PartialEq)]
pub struct HandlerError {
    pub code: ErrorCode,
    pub message: String,
    pub fields: Option<Value>,
    pub disk_failure: Option<String>,
}

impl HandlerError {
    #[must_use]
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::plain(ErrorCode::Invalid, message)
    }

    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::plain(ErrorCode::NotFound, message)
    }

    #[must_use]
    pub fn refused(message: impl Into<String>) -> Self {
        Self::plain(ErrorCode::Refused, message)
    }

    /// Failing fields, each with its rule.
    #[must_use]
    pub fn invalid_fields(errors: &[FieldError]) -> Self {
        let message = errors
            .iter()
            .map(|error| format!("{} {}", error.field, error.rule))
            .collect::<Vec<_>>()
            .join("; ");
        Self {
            fields: Some(field_errors_json(errors)),
            ..Self::invalid(message)
        }
    }

    fn plain(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            fields: None,
            disk_failure: None,
        }
    }
}

impl From<ScheduleApplicationError> for HandlerError {
    fn from(error: ScheduleApplicationError) -> Self {
        if let ScheduleApplicationError::Invalid(errors) = &error {
            return Self::invalid_fields(errors);
        }
        let message = error.to_string();
        let disk_failure = error.is_disk_failure().then(|| message.clone());
        let code = match error {
            ScheduleApplicationError::Invalid(_) => ErrorCode::Invalid,
            ScheduleApplicationError::NameTaken { .. } => ErrorCode::Conflict,
            ScheduleApplicationError::CapReached { .. }
            | ScheduleApplicationError::SwitchRefused(_)
            | ScheduleApplicationError::RunSettled(_) => ErrorCode::Refused,
            ScheduleApplicationError::NotFound(_) => ErrorCode::NotFound,
            ScheduleApplicationError::Storage { .. } => ErrorCode::Internal,
        };
        Self {
            code,
            message,
            fields: None,
            disk_failure,
        }
    }
}
