// SPDX-License-Identifier: AGPL-3.0-or-later

//! The answers of the read-only file view of an agent (0016).

use kernel::Timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Folder,
    File,
    Link,
    /// A pipe, a socket, or a device.
    Other,
}

/// Where a link leads, seen from the root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkTarget {
    Folder,
    File,
    Other,
    Outside,
    /// Missing, or a loop.
    Broken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderEntry {
    pub name: String,
    pub kind: EntryKind,
    pub size: Option<u64>,
    pub secret: bool,
    pub target: Option<LinkTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderListing {
    pub root: String,
    pub path: String,
    pub entries: Vec<FolderEntry>,
    pub more: usize,
    pub read_at: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentState {
    Secret,
    TooLarge,
    Binary,
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileContent {
    pub path: String,
    pub state: ContentState,
    pub size: u64,
    /// Only for `ContentState::Text`.
    pub bytes: Option<Vec<u8>>,
    pub read_at: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    TypeChanged,
    Untracked,
    Conflicted,
    Submodule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    pub path: String,
    pub kind: ChangeKind,
    pub old_path: Option<String>,
    pub secret: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeList {
    pub repository: bool,
    pub files: Vec<ChangedFile>,
    pub more: usize,
    pub read_at: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffState {
    Secret,
    TooLarge,
    Binary,
    None,
    Diff,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    pub path: String,
    pub state: DiffState,
    /// The size of the diff, when known.
    pub size: Option<u64>,
    /// Only for `DiffState::Diff`.
    pub bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitSummary {
    pub id: String,
    pub short_id: String,
    pub subject: String,
    pub author: String,
    pub time: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitPage {
    pub repository: bool,
    pub commits: Vec<CommitSummary>,
    pub more: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitDetail {
    pub id: String,
    pub parents: Vec<String>,
    pub author: String,
    pub email: String,
    pub time: Timestamp,
    pub message: String,
    pub files: Vec<ChangedFile>,
    pub more: usize,
}
