// SPDX-License-Identifier: AGPL-3.0-or-later

/// One live item for an attached client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveOutput {
    /// Raw terminal output, in order.
    Bytes(Vec<u8>),
    /// The output file passed its limit and dropped its oldest output.
    OlderOutputDropped,
}
