// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::Deserialize;

/// Body of `terminal.attach`.
#[derive(Debug, Deserialize)]
pub struct AttachRequest {
    pub agent_id: String,
    pub cols: u16,
    pub rows: u16,
}
