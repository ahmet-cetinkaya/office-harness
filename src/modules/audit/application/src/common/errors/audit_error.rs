// SPDX-License-Identifier: AGPL-3.0-or-later

use audit_domain::ActionError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("the action record failed: {message}")]
    Storage {
        message: String,
        /// True when the disk is at fault, which the server reports as
        /// `storage.error`.
        is_disk_failure: bool,
    },
    #[error(transparent)]
    Corrupt(#[from] ActionError),
}
