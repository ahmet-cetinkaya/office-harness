// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

/// The text never repeats the rejected value, because it could be a token.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TokenError {
    #[error(
        "the token is not {} lowercase hex characters",
        crate::value_objects::TOKEN_TEXT_LENGTH
    )]
    Malformed,
}
