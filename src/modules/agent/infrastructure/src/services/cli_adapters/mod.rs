// SPDX-License-Identifier: AGPL-3.0-or-later

//! One adapter for each supported CLI (D5, research R3). An adapter builds
//! command lines and reads the CLI's session files; it never writes a file
//! of the CLI (FR-021).

mod claude;
mod codex;
mod codex_sessions;
mod command_builder;
mod gemini;
mod generic;
mod opencode;
mod registry;

pub use codex_sessions::AttributionTiming;
pub use registry::CliAdapterRegistry;
