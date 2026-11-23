// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

/// Why a secret name or value is refused. No text holds a value.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SecretError {
    #[error("the secret name is empty")]
    NameEmpty,
    #[error("the secret name is longer than {max} characters")]
    NameTooLong { max: usize },
    #[error(
        "the secret name must start with a letter A to Z and hold only the letters A to Z, digits, and underscore"
    )]
    NameBadForm,
    #[error("the secret value is shorter than {min} bytes")]
    ValueTooShort { min: usize },
    #[error("the secret value is longer than {max} bytes")]
    ValueTooLong { max: usize },
    #[error("the secret value holds a NUL character")]
    ValueHasNul,
}
