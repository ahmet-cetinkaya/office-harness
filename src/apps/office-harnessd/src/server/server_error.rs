// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io;

use access_application::common::errors::AccessError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("cannot listen on the loopback interface: {0}")]
    Bind(io::Error),
    #[error("cannot write server.json: {0}")]
    ServerFile(io::Error),
    #[error("cannot make the start nonce: {0}")]
    StartNonce(AccessError),
    #[error("the server stopped with a failure: {0}")]
    Serve(io::Error),
}
