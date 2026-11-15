// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

/// A stored action text that no known action matches.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("unknown {field}: {text}")]
pub struct ActionError {
    pub field: &'static str,
    pub text: String,
}
