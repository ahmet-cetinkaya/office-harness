// SPDX-License-Identifier: AGPL-3.0-or-later

const OK_TEXT: &str = "ok";

/// How an action ended: stored as `ok`, or as the error text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionResult {
    Ok,
    Failed(String),
}

impl ActionResult {
    #[must_use]
    pub fn as_text(&self) -> &str {
        match self {
            Self::Ok => OK_TEXT,
            Self::Failed(message) => message,
        }
    }

    #[must_use]
    pub fn from_text(text: &str) -> Self {
        if text == OK_TEXT {
            Self::Ok
        } else {
            Self::Failed(text.to_owned())
        }
    }
}
