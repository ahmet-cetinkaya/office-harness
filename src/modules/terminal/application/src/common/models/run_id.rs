// SPDX-License-Identifier: AGPL-3.0-or-later

/// Names one start of an agent's terminal. Every start gets a new, larger
/// number, so a late event of an earlier run is told apart from the current
/// one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RunId(pub u64);
