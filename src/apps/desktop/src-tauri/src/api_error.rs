// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::Serialize;
use ts_rs::TS;

pub const NOT_CONNECTED: &str = "not_connected";
pub const START_FAILED: &str = "start_failed";
pub const START_TIMEOUT: &str = "start_timeout";
pub const LOGIN_START_FAILED: &str = "login_start_failed";
pub const WINDOW_TITLE_FAILED: &str = "window_title_failed";
pub const TEMPLATE_FILE_FAILED: &str = "template_file_failed";

/// The failure of a command, as the webview receives it. The code is one of
/// the server's error codes or one of the codes above, which the core raises
/// itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../src/core/api/generated/")]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }

    pub fn not_connected() -> Self {
        Self::new(NOT_CONNECTED, "The office server is not connected.")
    }
}
