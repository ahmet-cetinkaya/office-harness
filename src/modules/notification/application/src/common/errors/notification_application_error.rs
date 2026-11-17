// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

/// What a notification use case can fail with, mapped to the wire error codes.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum NotificationApplicationError {
    #[error("{0}")]
    NotFound(String),
    #[error("storage failure: {message}")]
    Storage {
        message: String,
        /// True when the disk is at fault, which the server reports as
        /// `storage.error` and survives.
        is_disk_failure: bool,
    },
}

impl NotificationApplicationError {
    /// True when the disk, not the request, is at fault.
    #[must_use]
    pub const fn is_disk_failure(&self) -> bool {
        matches!(
            self,
            Self::Storage {
                is_disk_failure: true,
                ..
            }
        )
    }
}
