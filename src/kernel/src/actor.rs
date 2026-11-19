// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

use crate::AgentId;

/// Who performed an action: the user, an agent (from release 0002), or the
/// office itself (a hold or release it applies on its own).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "agent_id")]
pub enum Actor {
    User,
    Agent(AgentId),
    Office,
}
