// SPDX-License-Identifier: AGPL-3.0-or-later

//! Terminal module protocol surface: the bodies of `terminal.attach`,
//! `terminal.detach`, `terminal.input`, and `terminal.resize`, the binary
//! frames, and the handlers that call the use cases. The host app owns the
//! socket and routes each message here by its prefix.

pub mod contracts;
pub mod handlers;
pub mod mappings;
