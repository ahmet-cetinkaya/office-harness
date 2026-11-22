// SPDX-License-Identifier: AGPL-3.0-or-later

//! Template module use cases: save, delete, list, copy an agent's definition,
//! export, and import with a preview. The store, the agent source, and the
//! clock are ports; the host app provides them. Creating agents from a
//! template crosses modules, so it lives in the host app (D5).

pub mod common;
pub mod services;

#[cfg(test)]
mod test_support;
