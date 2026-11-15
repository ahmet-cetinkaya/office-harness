// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::entities::AdditionsRecord;

/// An agent's additions, and whether an edit waits for the next session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdditionsView {
    pub record: AdditionsRecord,
    /// True while the agent runs: the running session keeps the values it
    /// started with, and the next session gets these (FR-023).
    pub applies_next_session: bool,
}
