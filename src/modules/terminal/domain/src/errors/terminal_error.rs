// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

/// A rule of the terminal module that an input broke. Messages name the field
/// at fault.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TerminalError {
    #[error("{field} must be greater than zero")]
    NotPositive { field: &'static str },
    #[error("{field} must not be empty")]
    Empty { field: &'static str },
    #[error("{field} must be an absolute path")]
    RelativePath { field: &'static str },
    #[error("{field} is not a valid identifier: {text}")]
    InvalidId { field: &'static str, text: String },
    #[error("{field} is not valid base64")]
    InvalidEncoding { field: &'static str },
    #[error("argument {index} contains {character:?}, which a batch script cannot receive safely")]
    UnsafeBatchArgument { index: usize, character: char },
    #[error(
        "the program path contains {character:?}, which a batch script cannot start with safely"
    )]
    UnsafeBatchProgram { character: char },
    #[error("output key must be 1 to {max} letters, digits, '-' or '_': {text}")]
    InvalidOutputKey { max: usize, text: String },
}
