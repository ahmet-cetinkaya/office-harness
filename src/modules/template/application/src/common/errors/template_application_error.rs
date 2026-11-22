// SPDX-License-Identifier: AGPL-3.0-or-later

use template_domain::TemplateError;
use thiserror::Error;

/// What a template use case can fail with, mapped to the wire error codes.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TemplateApplicationError {
    /// A rule of the template or of the file; the text names the reason.
    #[error(transparent)]
    Invalid(#[from] TemplateError),
    #[error("template name \"{name}\" is already used")]
    NameTaken { name: String },
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

impl TemplateApplicationError {
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
