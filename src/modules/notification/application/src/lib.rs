// SPDX-License-Identifier: AGPL-3.0-or-later

//! Notification module use cases: create, list, and read the rows the user
//! must see. The store port is declared here; the host app provides the
//! SQLite adapter.

pub mod commands;
pub mod common;
pub mod queries;

#[cfg(test)]
mod test_support;
