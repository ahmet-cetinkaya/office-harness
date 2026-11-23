// SPDX-License-Identifier: AGPL-3.0-or-later

//! Schedule module use cases: save, switch, delete, list, the round planner,
//! and the run book. The store, the clock, the cap, the target check, and the
//! zone are ports; the host app provides them.

pub mod common;
pub mod services;

#[cfg(test)]
mod test_support;
