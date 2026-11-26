// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const NOT_CONNECTED: &str = "not_connected";
pub const START_FAILED: &str = "start_failed";
pub const START_TIMEOUT: &str = "start_timeout";
pub const LOGIN_START_FAILED: &str = "login_start_failed";
pub const WINDOW_TITLE_FAILED: &str = "window_title_failed";
pub const TEMPLATE_FILE_FAILED: &str = "template_file_failed";

/// One failing field of an `invalid` answer and the rule it broke.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/core/api/generated/")]
pub struct FieldFailure {
    pub field: String,
    pub rule: String,
    pub message: String,
}

/// The failure of a command, as the webview receives it. The code is one of
/// the server's error codes or one of the codes above, which the core raises
/// itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../src/core/api/generated/")]
pub struct ApiError {
    pub code: String,
    pub message: String,
    /// The failing fields of an `invalid` answer; left out when the server
    /// names none.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fields: Option<Vec<FieldFailure>>,
}

impl ApiError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
            fields: None,
        }
    }

    #[must_use]
    pub fn with_fields(mut self, fields: Option<Vec<FieldFailure>>) -> Self {
        self.fields = fields;
        self
    }

    pub fn not_connected() -> Self {
        Self::new(NOT_CONNECTED, "The office server is not connected.")
    }
}
