// SPDX-License-Identifier: AGPL-3.0-or-later

//! Agent module use cases: agents, projects, settings, and the status that
//! terminal events drive. Ports for the stores, the terminal host, the clock,
//! and the action recorder are declared here; the host app implements them.

pub mod commands;
pub mod common;
pub mod queries;

#[cfg(test)]
mod test_support;
