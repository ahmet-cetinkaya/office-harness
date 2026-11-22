// SPDX-License-Identifier: AGPL-3.0-or-later

use template_application::common::errors::TemplateApplicationError;
use template_domain::TemplateError;
use wire::ErrorCode;

/// A failed request: the wire code and a message that names the field or
/// entry at fault. `disk_failure` is the text for `storage.error` when the
/// disk, not the request, is at fault.
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

impl From<TemplateError> for HandlerError {
    fn from(error: TemplateError) -> Self {
        Self::invalid(error.to_string())
    }
}

impl From<TemplateApplicationError> for HandlerError {
    fn from(error: TemplateApplicationError) -> Self {
        let message = error.to_string();
        let disk_failure = error.is_disk_failure().then(|| message.clone());
        let code = match error {
            TemplateApplicationError::Invalid(_) => ErrorCode::Invalid,
            TemplateApplicationError::NameTaken { .. } => ErrorCode::Conflict,
            TemplateApplicationError::NotFound(_) => ErrorCode::NotFound,
            TemplateApplicationError::Storage { .. } => ErrorCode::Internal,
        };
        Self {
            code,
            message,
            disk_failure,
        }
    }
}
