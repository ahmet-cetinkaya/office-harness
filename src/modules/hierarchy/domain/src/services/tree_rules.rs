// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;

use kernel::AgentId;

use crate::errors::HierarchyError;
use crate::value_objects::Role;

/// The place an agent takes, or does not take, in the company tree (FR-001,
/// FR-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub agent_id: AgentId,
    pub role: Role,
    pub parent_agent_id: Option<AgentId>,
    pub team_id: Option<crate::entities::TeamId>,
}

/// One row of the tree as storage reads it. The name travels with the row,
/// so a refusal can name the agent instead of its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRow {
    pub agent_id: AgentId,
    pub name: String,
    pub role: Option<Role>,
    pub parent_agent_id: Option<AgentId>,
    pub team_id: Option<crate::entities::TeamId>,
}

/// Checks a placement against the tree rules. The tree is the rows the office
/// holds now; the caller has already written nothing.
///
/// # Errors
/// `SecondCeo` when a CEO exists; `SecondLead` when the placement's team
/// already holds a lead; `ParentNotFound` when the parent is not in the
/// tree; `RoleNotAllowed` naming the allowed roles; `Cycle` and `TooDeep`
/// for a chain that cannot exist.
pub fn check_placement(tree: &[TreeRow], placement: Placement) -> Result<(), HierarchyError> {
    match placement.role {
        Role::Ceo => check_ceo(tree, placement.agent_id),
        Role::Lead => check_lead(tree, placement),
        Role::Member => check_member(tree, placement),
    }
}

fn check_ceo(tree: &[TreeRow], agent_id: AgentId) -> Result<(), HierarchyError> {
    if let Some(existing) = tree.iter().find(|row| row.role == Some(Role::Ceo)) {
        // The same agent keeps its own role on a re-check.
        if existing.agent_id != agent_id {
            return Err(HierarchyError::SecondCeo {
                // An empty name reads as the id, the only text the store
                // guarantees (the agent module owns the name column).
                name: non_empty_name_of(existing),
            });
        }
    }
    Ok(())
}

fn check_lead(tree: &[TreeRow], placement: Placement) -> Result<(), HierarchyError> {
    // A lead's team is its place; its parent is the CEO when one exists
    // (A5). The caller passes no parent for a lead.
    if let Some(parent_id) = placement.parent_agent_id {
        let parent = row_of(tree, parent_id)?;
        if parent.role != Some(Role::Ceo) {
            return Err(role_not_allowed(Role::Lead, Role::Ceo));
        }
    }
    if let Some(parent) = ceo_of(tree)
        && parent.agent_id == placement.agent_id
    {
        return Err(role_not_allowed(Role::Lead, Role::Ceo));
    }
    // One lead per team: a team already holding one refuses the second,
    // naming the lead at the seat (the CEO refusal's own shape). The team
    // row alone cannot tell this — its lead column can name an agent whose
    // row has left the tree — so the live rows are the seat's truth.
    if let Some(team_id) = placement.team_id
        && let Some(lead) = tree
            .iter()
            .find(|row| row.role == Some(Role::Lead) && row.team_id == Some(team_id))
        && lead.agent_id != placement.agent_id
    {
        return Err(HierarchyError::SecondLead {
            name: non_empty_name_of(lead),
        });
    }
    Ok(())
}

fn check_member(tree: &[TreeRow], placement: Placement) -> Result<(), HierarchyError> {
    let parent_id = placement
        .parent_agent_id
        .ok_or(HierarchyError::LeadMissing)?;
    if parent_id == placement.agent_id {
        return Err(HierarchyError::Cycle);
    }
    let parent = row_of(tree, parent_id)?;
    if parent.role != Some(Role::Lead) {
        return Err(role_not_allowed(Role::Member, Role::Lead));
    }
    // A member's team is its lead's team (A6); the caller passes none.
    if let Some(team) = placement.team_id
        && Some(team) != parent.team_id
    {
        return Err(HierarchyError::NotInHierarchy);
    }
    Ok(())
}

/// The parent an agent takes, resolved for storage: a lead's parent is the
/// CEO when one exists, else none; a member's parent is its lead.
#[must_use]
pub fn resolved_parent(tree: &[TreeRow], placement: Placement) -> Option<AgentId> {
    match placement.role {
        Role::Ceo => None,
        Role::Lead => placement.parent_agent_id.or_else(|| {
            ceo_of(tree)
                .filter(|ceo| ceo.agent_id != placement.agent_id)
                .map(|ceo| ceo.agent_id)
        }),
        Role::Member => placement.parent_agent_id,
    }
}

/// The descendants of an agent, deepest level first so a caller can stop or
/// delete a subtree from the leaves up. Inside a level the order follows the
/// rows. The agent itself is not in the list. Each agent is visited once, so
/// a hand-made cycle in the rows cannot extend the walk.
#[must_use]
pub fn descendants_of(tree: &[TreeRow], agent_id: AgentId) -> Vec<AgentId> {
    let mut seen = HashSet::from([agent_id]);
    let mut levels: Vec<Vec<AgentId>> = Vec::new();
    let mut frontier = vec![agent_id];
    while !frontier.is_empty() {
        let mut next = Vec::new();
        for current in &frontier {
            for row in tree {
                if row.parent_agent_id == Some(*current) && seen.insert(row.agent_id) {
                    next.push(row.agent_id);
                }
            }
        }
        if !next.is_empty() {
            levels.push(next.clone());
        }
        frontier = next;
    }
    levels.iter().rev().flatten().copied().collect()
}

/// Whether `agent_id` is a descendant of `ancestor_id` (FR-008, FR-009).
/// The walk is capped at the row count: an honest chain never needs more
/// steps than there are rows, so the cap turns a hand-made cycle into a
/// plain no instead of a hang.
#[must_use]
pub fn is_descendant_of(tree: &[TreeRow], agent_id: AgentId, ancestor_id: AgentId) -> bool {
    let mut current = agent_id;
    for _ in 0..=tree.len() {
        let Some(row) = tree.iter().find(|row| row.agent_id == current) else {
            return false;
        };
        match row.parent_agent_id {
            Some(parent) if parent == ancestor_id => return true,
            Some(parent) => current = parent,
            None => return false,
        }
    }
    false
}

fn ceo_of(tree: &[TreeRow]) -> Option<TreeRow> {
    tree.iter().find(|row| row.role == Some(Role::Ceo)).cloned()
}

fn row_of(tree: &[TreeRow], agent_id: AgentId) -> Result<TreeRow, HierarchyError> {
    tree.iter()
        .find(|row| row.agent_id == agent_id)
        .cloned()
        .ok_or(HierarchyError::ParentNotFound)
}

/// The name a refusal shows: the stored name when it says something, else
/// the id.
fn non_empty_name_of(row: &TreeRow) -> String {
    if row.name.is_empty() {
        row.agent_id.to_string()
    } else {
        row.name.clone()
    }
}

fn role_not_allowed(child: Role, parent: Role) -> HierarchyError {
    let allowed = parent
        .allowed_child_roles()
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    HierarchyError::RoleNotAllowed {
        child,
        parent,
        allowed,
    }
}

#[cfg(test)]
#[path = "tree_rules.tests.rs"]
mod tests;
