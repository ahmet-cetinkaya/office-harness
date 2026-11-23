// SPDX-License-Identifier: AGPL-3.0-or-later

use secret_application::SecretServiceError;
use wire::ErrorCode;

/// The wire code of a service error (contract).
pub(crate) fn code_of(error: &SecretServiceError) -> ErrorCode {
    match error {
        SecretServiceError::Invalid(_) | SecretServiceError::TooManySecrets { .. } => {
            ErrorCode::Invalid
        }
        SecretServiceError::NoSuchSecret { .. }
        | SecretServiceError::NoSuchAgent { .. }
        | SecretServiceError::NotAvailable { .. } => ErrorCode::NotFound,
        SecretServiceError::AlreadyGranted
        | SecretServiceError::NotGranted
        | SecretServiceError::ConfirmCountDiffers { .. } => ErrorCode::Conflict,
        SecretServiceError::Locked(_) => ErrorCode::Refused,
        SecretServiceError::CannotRead { .. } | SecretServiceError::Storage { .. } => {
            ErrorCode::Internal
        }
    }
}
