// SPDX-License-Identifier: AGPL-3.0-or-later

//! The company tree of the office: roles, teams, and the rules that keep
//! the tree a CEO > team lead > member hierarchy.

pub mod entities;
pub mod errors;
pub mod services;
pub mod value_objects;

pub use entities::{Team, TeamId};
pub use errors::HierarchyError;
pub use services::tree_rules::{Placement, TreeRow, check_placement, descendants_of, is_descendant_of, resolved_parent};
pub use value_objects::{Role, TeamName, parse_role};
