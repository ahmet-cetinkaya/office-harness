// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;

use crate::errors::HierarchyError;
use crate::services::tree_rules::{TreeRow, is_descendant_of};

/// The longest text a directive may carry.
pub const DIRECTIVE_MAX_CHARS: usize = 8192;

/// Lets `caller` act on `target` only when the target sits in the caller's
/// subtree; an agent never acts on itself.
///
/// # Errors
/// `NotADescendant` when the target is the caller itself or outside the
/// caller's subtree.
pub fn check_descendant(
    tree: &[TreeRow],
    caller: AgentId,
    target: AgentId,
) -> Result<(), HierarchyError> {
    if target != caller && is_descendant_of(tree, target, caller) {
        Ok(())
    } else {
        Err(HierarchyError::NotADescendant {
            actor: caller.to_string(),
        })
    }
}

/// Resolves the parent a new agent made by `caller` gets: the requested
/// parent, or the caller itself when none is requested. The parent must stay
/// inside the caller's subtree; the role rules stay in `check_placement`.
///
/// # Errors
/// `NotADescendant` when the requested parent is outside the caller's
/// subtree.
pub fn creation_parent(
    tree: &[TreeRow],
    caller: AgentId,
    requested: Option<AgentId>,
) -> Result<AgentId, HierarchyError> {
    let parent = requested.unwrap_or(caller);
    if parent == caller || is_descendant_of(tree, parent, caller) {
        Ok(parent)
    } else {
        Err(HierarchyError::NotADescendant {
            actor: caller.to_string(),
        })
    }
}

/// Accepts directive text that says something, fits the length bound, and
/// carries no control characters (C0, DEL, C1).
///
/// # Errors
/// `Empty` for blank text; `TooLong` past `DIRECTIVE_MAX_CHARS`;
/// `ControlCharacter` for any control character.
pub fn validate_directive_text(text: &str) -> Result<(), HierarchyError> {
    if text.trim().is_empty() {
        return Err(HierarchyError::Empty { field: "text" });
    }
    if text.chars().count() > DIRECTIVE_MAX_CHARS {
        return Err(HierarchyError::TooLong {
            field: "text",
            max: DIRECTIVE_MAX_CHARS,
        });
    }
    if text.chars().any(char::is_control) {
        return Err(HierarchyError::ControlCharacter { field: "text" });
    }
    Ok(())
}

#[cfg(test)]
#[path = "call_rules.tests.rs"]
mod tests;
