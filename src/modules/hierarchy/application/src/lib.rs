// SPDX-License-Identifier: AGPL-3.0-or-later

//! The use cases of the hierarchy: teams, placements, and the tree query.

pub mod commands;
pub mod common;
pub mod queries;

pub use common::errors::HierarchyApplicationError;

pub use commands::create_role_agent::{CreateRoleAgentHandler, CreateRoleAgentRequest};
pub use commands::create_team::CreateTeamHandler;
pub use commands::delete_team::DeleteTeamHandler;
pub use commands::place_agent::PlaceAgentHandler;
pub use commands::send_directive::SendDirectiveHandler;
pub use commands::subtree::SubtreeHandler;
pub use queries::tree::TreeQuery;
