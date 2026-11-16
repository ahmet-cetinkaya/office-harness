// SPDX-License-Identifier: AGPL-3.0-or-later

/// The protocol error codes that a terminal message can end with. The host
/// maps each to the code of its wire envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Invalid,
    Conflict,
    NotFound,
    Internal,
}
