// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

/// Why sealed secrets exist and the store key cannot be had (contract:
/// `locked_reason`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockedReason {
    KeyringLocked,
    KeyringNotRunning,
    KeyringEntryMissing,
    KeyFileMissing,
}

impl fmt::Display for LockedReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::KeyringLocked => "the keyring is locked",
            Self::KeyringNotRunning => "the keyring is not running",
            Self::KeyringEntryMissing => "the keyring entry is missing",
            Self::KeyFileMissing => "key file missing",
        })
    }
}
