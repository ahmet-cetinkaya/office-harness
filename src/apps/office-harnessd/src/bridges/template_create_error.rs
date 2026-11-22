// SPDX-License-Identifier: AGPL-3.0-or-later

use hierarchy_application::common::errors::HierarchyApplicationError;
use hierarchy_application::common::ports::{CreatorError, CreatorErrorKind};
use hierarchy_domain::HierarchyError;
use wire::ErrorCode;

/// Why a creation from a template failed: the wire code and a message that
/// names every reason, one per line (FR-005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateCreateError {
    pub code: ErrorCode,
    pub message: String,
}

impl TemplateCreateError {
    /// Every reason that the checks found, one per line.
    #[must_use]
    pub fn invalid(reasons: &[String]) -> Self {
        Self {
            code: ErrorCode::Invalid,
            message: reasons.join("\n"),
        }
    }

    #[must_use]
    pub fn conflict(message: String) -> Self {
        Self {
            code: ErrorCode::Conflict,
            message,
        }
    }

    #[must_use]
    pub fn not_found(message: String) -> Self {
        Self {
            code: ErrorCode::NotFound,
            message,
        }
    }

    #[must_use]
    pub fn internal(message: String) -> Self {
        Self {
            code: ErrorCode::Internal,
            message,
        }
    }

    /// A tree rule that the place breaks. The text is the 0002 rule's own,
    /// and the code is `refused`, except for a parent that does not exist.
    #[must_use]
    pub fn from_tree_rule(rule: &HierarchyError) -> Self {
        let code = match rule {
            HierarchyError::ParentNotFound => ErrorCode::NotFound,
            _ => ErrorCode::Refused,
        };
        Self {
            code,
            message: rule.to_string(),
        }
    }

    /// The agent module's own refusal or failure, with its code.
    #[must_use]
    pub fn from_creator(error: CreatorError) -> Self {
        let code = match error.kind {
            CreatorErrorKind::Invalid => ErrorCode::Invalid,
            CreatorErrorKind::NotFound => ErrorCode::NotFound,
            CreatorErrorKind::Conflict => ErrorCode::Conflict,
            CreatorErrorKind::Internal => ErrorCode::Internal,
        };
        Self {
            code,
            message: error.message,
        }
    }

    #[must_use]
    pub fn from_hierarchy(error: &HierarchyApplicationError) -> Self {
        Self::internal(error.to_string())
    }

    /// The same failure with what the undo could not remove, so the user can
    /// find and delete it.
    #[must_use]
    pub fn with_leftovers(mut self, leftovers: &[String]) -> Self {
        for leftover in leftovers {
            self.message.push_str("; ");
            self.message.push_str(leftover);
        }
        self
    }
}
