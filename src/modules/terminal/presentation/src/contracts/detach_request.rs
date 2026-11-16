// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::Deserialize;

/// Body of `terminal.detach`.
#[derive(Debug, Deserialize)]
pub struct DetachRequest {
    pub agent_id: String,
}
