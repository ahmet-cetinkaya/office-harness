// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::Deserialize;

/// Body of `terminal.input`. `data` is base64 of the bytes to type.
#[derive(Debug, Deserialize)]
pub struct InputRequest {
    pub agent_id: String,
    pub data: String,
}
