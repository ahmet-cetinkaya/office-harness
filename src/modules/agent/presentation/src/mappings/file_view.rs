// SPDX-License-Identifier: AGPL-3.0-or-later

//! The JSON of the `agent.files.*` answers (0016 contract): base64 content
//! and lists that stop at the answer budget.

use agent_application::common::models::{
    ChangeKind, ChangeList, ChangedFile, CommitDetail, CommitPage, CommitSummary, ContentState,
    DiffState, EntryKind, FileContent, FileDiff, FolderEntry, FolderListing, LinkTarget,
};
use agent_domain::services::LIST_BUDGET_BYTES;
use serde_json::{Value, json};

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const BASE64_PADDING: char = '=';
const BYTES_PER_GROUP: usize = 3;
const CHARACTERS_PER_GROUP: usize = 4;
const BITS_PER_CHARACTER: usize = 6;
const CHARACTER_MASK: u32 = 0b11_1111;
/// The comma between two list items.
const SEPARATOR_BYTES: usize = 1;

/// `agent.files.list`.
#[must_use]
pub fn listing_json(listing: &FolderListing) -> Value {
    let (entries, left_out) = within_budget(
        listing.entries.iter().map(entry_json).collect(),
        LIST_BUDGET_BYTES,
    );
    json!({
        "root": listing.root,
        "path": listing.path,
        "entries": entries,
        "more": listing.more + left_out,
        "read_at": listing.read_at.as_unix_millis(),
    })
}

/// `agent.files.read`.
#[must_use]
pub fn content_json(content: &FileContent) -> Value {
    json!({
        "path": content.path,
        "state": content_state(content.state),
        "size": content.size,
        "content_base64": content.bytes.as_deref().map(base64_encode),
        "read_at": content.read_at.as_unix_millis(),
    })
}

/// `agent.files.changes`.
#[must_use]
pub fn changes_json(changes: &ChangeList) -> Value {
    let (files, left_out) = changed_files_json(&changes.files, LIST_BUDGET_BYTES);
    json!({
        "repository": changes.repository,
        "files": files,
        "more": changes.more + left_out,
        "read_at": changes.read_at.as_unix_millis(),
    })
}

/// `agent.files.diff`.
#[must_use]
pub fn diff_json(diff: &FileDiff) -> Value {
    json!({
        "path": diff.path,
        "state": diff_state(diff.state),
        "size": diff.size,
        "content_base64": diff.bytes.as_deref().map(base64_encode),
    })
}

/// `agent.files.history`.
#[must_use]
pub fn history_json(page: &CommitPage) -> Value {
    let (commits, left_out) = within_budget(
        page.commits.iter().map(commit_summary_json).collect(),
        LIST_BUDGET_BYTES,
    );
    json!({
        "repository": page.repository,
        "commits": commits,
        "more": page.more || left_out > 0,
    })
}

/// `agent.files.commit`. The message counts against the budget of the
/// file list, so the whole answer stays under the frame limit.
#[must_use]
pub fn commit_json(detail: &CommitDetail) -> Value {
    let message_size = Value::from(detail.message.as_str()).to_string().len();
    let (files, left_out) = changed_files_json(
        &detail.files,
        LIST_BUDGET_BYTES.saturating_sub(message_size),
    );
    json!({
        "id": detail.id,
        "parents": detail.parents,
        "author": detail.author,
        "email": detail.email,
        "time": detail.time.as_unix_millis(),
        "message": detail.message,
        "files": files,
        "more": detail.more + left_out,
    })
}

/// The bytes as standard base64 with padding.
#[must_use]
pub fn base64_encode(bytes: &[u8]) -> String {
    let mut encoded =
        String::with_capacity(bytes.len().div_ceil(BYTES_PER_GROUP) * CHARACTERS_PER_GROUP);
    for group in bytes.chunks(BYTES_PER_GROUP) {
        let mut padded = [0_u8; BYTES_PER_GROUP];
        padded[..group.len()].copy_from_slice(group);
        let packed = u32::from_be_bytes([0, padded[0], padded[1], padded[2]]);
        // A group of n bytes makes n + 1 characters; padding fills the rest.
        for index in 0..=group.len() {
            let shift = BITS_PER_CHARACTER * (CHARACTERS_PER_GROUP - 1 - index);
            let position = usize::try_from((packed >> shift) & CHARACTER_MASK).unwrap_or_default();
            encoded.push(char::from(BASE64_ALPHABET[position]));
        }
        for _ in group.len()..BYTES_PER_GROUP {
            encoded.push(BASE64_PADDING);
        }
    }
    encoded
}

/// The items that fit in `budget` bytes of JSON, in order, and how many
/// were left out. The first item that does not fit ends the list, so the
/// order and the paging stay plain.
fn within_budget(items: Vec<Value>, budget: usize) -> (Vec<Value>, usize) {
    let total = items.len();
    let mut used = 0;
    let kept: Vec<Value> = items
        .into_iter()
        .take_while(|item| {
            used += item.to_string().len() + SEPARATOR_BYTES;
            used <= budget
        })
        .collect();
    let left_out = total - kept.len();
    (kept, left_out)
}

fn changed_files_json(files: &[ChangedFile], budget: usize) -> (Vec<Value>, usize) {
    within_budget(files.iter().map(changed_file_json).collect(), budget)
}

fn entry_json(entry: &FolderEntry) -> Value {
    json!({
        "name": entry.name,
        "kind": entry_kind(entry.kind),
        "size": entry.size,
        "secret": entry.secret,
        "target": entry.target.map(link_target),
    })
}

fn changed_file_json(file: &ChangedFile) -> Value {
    json!({
        "path": file.path,
        "kind": change_kind(file.kind),
        "old_path": file.old_path,
        "secret": file.secret,
    })
}

fn commit_summary_json(commit: &CommitSummary) -> Value {
    json!({
        "id": commit.id,
        "short_id": commit.short_id,
        "subject": commit.subject,
        "author": commit.author,
        "time": commit.time.as_unix_millis(),
    })
}

fn entry_kind(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Folder => "folder",
        EntryKind::File => "file",
        EntryKind::Link => "link",
        EntryKind::Other => "other",
    }
}

fn link_target(target: LinkTarget) -> &'static str {
    match target {
        LinkTarget::Folder => "folder",
        LinkTarget::File => "file",
        LinkTarget::Other => "other",
        LinkTarget::Outside => "outside",
        LinkTarget::Broken => "broken",
    }
}

fn content_state(state: ContentState) -> &'static str {
    match state {
        ContentState::Secret => "secret",
        ContentState::TooLarge => "too_large",
        ContentState::Binary => "binary",
        ContentState::Text => "text",
    }
}

fn diff_state(state: DiffState) -> &'static str {
    match state {
        DiffState::Secret => "secret",
        DiffState::TooLarge => "too_large",
        DiffState::Binary => "binary",
        DiffState::None => "none",
        DiffState::Diff => "diff",
    }
}

fn change_kind(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Modified => "modified",
        ChangeKind::Added => "added",
        ChangeKind::Deleted => "deleted",
        ChangeKind::Renamed => "renamed",
        ChangeKind::TypeChanged => "type_changed",
        ChangeKind::Untracked => "untracked",
        ChangeKind::Conflicted => "conflicted",
        ChangeKind::Submodule => "submodule",
    }
}

#[cfg(test)]
#[path = "file_view.tests.rs"]
mod tests;
