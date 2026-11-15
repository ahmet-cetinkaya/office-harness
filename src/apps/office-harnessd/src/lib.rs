// SPDX-License-Identifier: AGPL-3.0-or-later

//! The background server host: configuration, the startup steps, and the
//! loopback WebSocket server. `main.rs`
//! is the composition root and stays thin.

pub mod bootstrap;
pub mod config;
pub mod server;
