// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("unknown argument: {0}")]
    UnknownArgument(String),
    #[error("{0} needs a value")]
    MissingValue(&'static str),
    #[error("the user's data folder cannot be determined: no home folder")]
    NoHomeFolder,
}
