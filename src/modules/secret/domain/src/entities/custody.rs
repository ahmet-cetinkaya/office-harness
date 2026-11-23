// SPDX-License-Identifier: AGPL-3.0-or-later

/// Where the store key lives (spec FR-003). Decided once, when the key is
/// created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Custody {
    OsKeyring,
    KeyFile,
}

impl Custody {
    /// The wire and storage text.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OsKeyring => "os_keyring",
            Self::KeyFile => "key_file",
        }
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "os_keyring" => Some(Self::OsKeyring),
            "key_file" => Some(Self::KeyFile),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "custody.tests.rs"]
mod tests;
