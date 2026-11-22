// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

/// Every branch the office makes starts with this, so the user can tell
/// them from his own (0010 D3).
pub const BRANCH_PREFIX: &str = "office-harness/";
const SLUG_MAX_CHARS: usize = 40;
const EMPTY_SLUG: &str = "agent";
const SLUG_SEPARATOR: char = '-';
/// The first suffix of a taken name: `<slug>-2` follows `<slug>`.
const FIRST_SUFFIX: usize = 2;
/// A SHA-1 and a SHA-256 commit id, in hex.
const COMMIT_ID_LENGTHS: [usize; 2] = [40, 64];
const BRANCH_REF_PREFIX: &str = "refs/heads/";

/// The own branch and worktree of an agent (0010 FR-002, FR-003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentWorktree {
    /// The top folder of the repository the agent was created in.
    pub repository: PathBuf,
    /// The branch the agent's branch started from, or the full commit id
    /// when the chosen folder had no branch checked out (A2).
    pub base: String,
    pub branch: String,
    /// The worktree folder, under the office's own worktrees folder.
    pub folder: PathBuf,
}

impl AgentWorktree {
    /// The base as git names it: `refs/heads/<base>` for a branch, the
    /// commit id itself otherwise. Never the raw text, so a base that starts
    /// with `-` cannot read as an option.
    #[must_use]
    pub fn base_ref(&self) -> String {
        base_ref(&self.base)
    }

    #[must_use]
    pub fn branch_ref(&self) -> String {
        format!("{BRANCH_REF_PREFIX}{}", self.branch)
    }
}

/// See `AgentWorktree::base_ref`.
#[must_use]
pub fn base_ref(base: &str) -> String {
    if is_commit_id(base) {
        base.to_owned()
    } else {
        format!("{BRANCH_REF_PREFIX}{base}")
    }
}

/// True for a full commit id: 40 or 64 lowercase hex digits.
#[must_use]
pub fn is_commit_id(text: &str) -> bool {
    COMMIT_ID_LENGTHS.contains(&text.len())
        && text
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character))
}

/// The agent name as a branch name part: lower-case `a-z`, `0-9`, and single
/// `-`, at most 40 characters, `agent` when nothing is left. That rules out
/// every sequence git refuses in a branch name (`..`, `@{`, `.lock`, `/`).
#[must_use]
pub fn branch_slug(agent_name: &str) -> String {
    let mut slug = String::new();
    for character in agent_name.chars().flat_map(char::to_lowercase) {
        let kept = if character.is_ascii_lowercase() || character.is_ascii_digit() {
            character
        } else {
            SLUG_SEPARATOR
        };
        if kept == SLUG_SEPARATOR && (slug.is_empty() || slug.ends_with(SLUG_SEPARATOR)) {
            continue;
        }
        slug.push(kept);
    }
    let slug: String = slug.chars().take(SLUG_MAX_CHARS).collect();
    let slug = slug.trim_end_matches(SLUG_SEPARATOR);
    if slug.is_empty() {
        EMPTY_SLUG.to_owned()
    } else {
        slug.to_owned()
    }
}

/// `office-harness/<slug>`, or the lowest free `office-harness/<slug>-<n>`
/// when `existing` holds it (0010 D3).
#[must_use]
pub fn free_branch_name(agent_name: &str, existing: &[String]) -> String {
    let slug = branch_slug(agent_name);
    let taken = |name: &str| existing.iter().any(|branch| branch == name);
    let first = format!("{BRANCH_PREFIX}{slug}");
    if !taken(&first) {
        return first;
    }
    // One more candidate than there are names, so one of them is free.
    (FIRST_SUFFIX..=FIRST_SUFFIX + existing.len())
        .map(|suffix| format!("{first}{SLUG_SEPARATOR}{suffix}"))
        .find(|name| !taken(name))
        .unwrap_or(first)
}

/// True when `name` has the shape `free_branch_name` makes: the prefix and
/// a slug, with an optional numeric suffix inside the slug's alphabet.
#[must_use]
pub fn is_office_branch_name(name: &str) -> bool {
    let Some(slug) = name.strip_prefix(BRANCH_PREFIX) else {
        return false;
    };
    let suffix_room = slug
        .rsplit_once(SLUG_SEPARATOR)
        .filter(|(_, suffix)| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()))
        .map_or(0, |(_, suffix)| suffix.len() + 1);
    !slug.is_empty()
        && slug.len() <= SLUG_MAX_CHARS + suffix_room
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == SLUG_SEPARATOR)
        && !slug.starts_with(SLUG_SEPARATOR)
        && !slug.ends_with(SLUG_SEPARATOR)
        && !slug.contains("--")
}

#[cfg(test)]
#[path = "agent_worktree.tests.rs"]
mod tests;
