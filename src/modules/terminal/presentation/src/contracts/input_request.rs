// SPDX-License-Identifier: AGPL-3.0-or-later

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use terminal_domain::errors::TerminalError;

/// Body of `terminal.input`. `data` is base64 of the bytes to type.
#[derive(Debug, Deserialize)]
pub struct InputRequest {
    pub agent_id: String,
    pub data: String,
}

impl InputRequest {
    /// The keys to type, decoded from `data`.
    ///
    /// # Errors
    /// `InvalidEncoding` when `data` is not base64.
    pub fn bytes(&self) -> Result<Vec<u8>, TerminalError> {
        STANDARD
            .decode(&self.data)
            .map_err(|_| TerminalError::InvalidEncoding { field: "data" })
    }
}
