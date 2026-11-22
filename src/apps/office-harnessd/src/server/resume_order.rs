// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use hierarchy_application::queries::tree::RowPlace;
use hierarchy_domain::Role;
use kernel::AgentId;

/// The start order of a bulk resume (0013 FR-003, A6): members and agents
/// outside the tree first, then leads, then the CEO, so that a directive a
/// resumed parent sends at once finds its targets running. The sort is
/// stable: within one level the order of `listed` stays.
#[must_use]
pub fn start_order(listed: &[AgentId], places: &HashMap<AgentId, RowPlace>) -> Vec<AgentId> {
    let mut ordered = listed.to_vec();
    ordered.sort_by_key(|agent_id| level_of(places.get(agent_id).and_then(|place| place.role)));
    ordered
}

/// Lower starts earlier.
fn level_of(role: Option<Role>) -> u8 {
    match role {
        None | Some(Role::Member) => 0,
        Some(Role::Lead) => 1,
        Some(Role::Ceo) => 2,
    }
}

#[cfg(test)]
#[path = "resume_order.tests.rs"]
mod tests;
