// SPDX-License-Identifier: AGPL-3.0-or-later

//! The pure rules of the file view (0016): the form of a path, which names
//! look secret, which content is binary, and the limits.

use crate::errors::AgentError;

/// The largest file content or diff the view shows: 512 KiB.
pub const MAX_CONTENT_BYTES: usize = 524_288;
pub const MAX_LIST_ENTRIES: usize = 1_000;
pub const MAX_CHANGED_FILES: usize = 1_000;
pub const COMMITS_PER_PAGE: usize = 50;
/// Status output of git beyond this is refused, not cut.
pub const MAX_STATUS_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
/// A list answer stops adding items before its JSON would pass this, so the
/// frame stays under the 1 MiB limit of the wire.
pub const LIST_BUDGET_BYTES: usize = 768 * 1024;
pub const MAX_SUBJECT_CHARS: usize = 200;
pub const MAX_MESSAGE_BYTES: usize = 65_536;
/// A file is binary when a zero byte is in this many leading bytes.
pub const BINARY_PREFIX_BYTES: usize = 8_000;
pub const MAX_SKIP: u32 = 1_000_000;

const PART_SEPARATOR: char = '/';
const CURRENT_PART: &str = ".";
const PARENT_PART: &str = "..";
const WINDOWS_FORBIDDEN: [char; 2] = ['\\', ':'];

const EXACT_SECRET_NAMES: [&str; 8] = [
    ".env",
    ".netrc",
    ".npmrc",
    ".pypirc",
    ".pgpass",
    ".git-credentials",
    ".htpasswd",
    "kubeconfig",
];
const SECRET_PREFIXES: [&str; 5] = [".env.", "id_rsa", "id_dsa", "id_ecdsa", "id_ed25519"];
const SECRET_SUFFIXES: [&str; 11] = [
    ".pem",
    ".key",
    ".p12",
    ".pfx",
    ".jks",
    ".keystore",
    ".kdbx",
    ".ppk",
    ".tfstate",
    ".ovpn",
    ".asc",
];
const SECRET_WORDS: [&str; 2] = ["secret", "credential"];

/// A path below the root, as the protocol sends it: the parts between `/`,
/// none of them empty, `.`, or `..`. The root itself has no part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelativePath {
    parts: Vec<String>,
}

impl RelativePath {
    /// The root.
    #[must_use]
    pub fn root() -> Self {
        Self { parts: Vec::new() }
    }

    /// A path of at least one part.
    ///
    /// # Errors
    /// `InvalidFilePath` for an empty path or a path of a refused form.
    pub fn parse(field: &'static str, text: &str) -> Result<Self, AgentError> {
        Self::parse_for(field, text, cfg!(windows))
    }

    /// Like `parse`, but the empty text is the root.
    ///
    /// # Errors
    /// `InvalidFilePath` for a path of a refused form.
    pub fn parse_folder(field: &'static str, text: &str) -> Result<Self, AgentError> {
        if text.is_empty() {
            return Ok(Self::root());
        }
        Self::parse(field, text)
    }

    /// `parse` with the platform choice given, so the Windows forms are
    /// testable everywhere.
    pub(crate) fn parse_for(
        field: &'static str,
        text: &str,
        is_windows: bool,
    ) -> Result<Self, AgentError> {
        let refuse = |reason| Err(AgentError::InvalidFilePath { field, reason });
        if text.is_empty() {
            return refuse("must not be empty");
        }
        if text.contains('\0') {
            return refuse("must not contain a NUL character");
        }
        if is_windows && text.contains(WINDOWS_FORBIDDEN) {
            return refuse("must not contain a backslash or a colon");
        }
        if text.starts_with(PART_SEPARATOR) {
            return refuse("must be relative to the working folder");
        }
        let parts: Vec<String> = text.split(PART_SEPARATOR).map(str::to_owned).collect();
        if parts.iter().any(String::is_empty) {
            return refuse("must not have an empty part");
        }
        if parts
            .iter()
            .any(|part| part == CURRENT_PART || part == PARENT_PART)
        {
            return refuse("must not have a . or .. part");
        }
        Ok(Self { parts })
    }

    #[must_use]
    pub fn parts(&self) -> &[String] {
        &self.parts
    }

    #[must_use]
    pub fn is_root(&self) -> bool {
        self.parts.is_empty()
    }

    /// The last part; `None` for the root.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.parts.last().map(String::as_str)
    }

    /// The path as the protocol writes it.
    #[must_use]
    pub fn to_text(&self) -> String {
        self.parts.join("/")
    }
}

/// True when the name, compared without regard to case, is one that often
/// holds a key or a password (spec Terms: secret-looking file).
#[must_use]
pub fn is_secret_looking(name: &str) -> bool {
    let name = name.to_lowercase();
    EXACT_SECRET_NAMES.contains(&name.as_str())
        || SECRET_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
        || SECRET_SUFFIXES.iter().any(|suffix| name.ends_with(suffix))
        || SECRET_WORDS.iter().any(|word| name.contains(word))
}

/// True when a zero byte is in the first 8,000 bytes of `content`.
#[must_use]
pub fn is_binary(content: &[u8]) -> bool {
    content
        .iter()
        .take(BINARY_PREFIX_BYTES)
        .any(|byte| *byte == 0)
}

#[cfg(test)]
#[path = "file_rules.tests.rs"]
mod tests;
