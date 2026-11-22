// SPDX-License-Identifier: AGPL-3.0-or-later

//! The company tree of the office: roles, teams, and the rules that keep
//! the tree a CEO > team lead > member hierarchy.

pub mod entities;
pub mod errors;
pub mod services;
pub mod value_objects;

pub use entities::{Team, TeamId};
pub use errors::HierarchyError;
pub use services::auto_hold_tracker::{AUTO_HOLD_REFUSAL_COUNT, AutoHoldTracker};
pub use services::call_rules::{
    DIRECTIVE_MAX_CHARS, check_descendant, creation_parent, validate_directive_text,
};
pub use services::hold::{Hold, HoldRefusal, HoldScope, HoldSetter, hold_refusal};
pub use services::rate_limits::{LimitKind, LimitRefusal, RATE_WINDOW_SECONDS, RateLimits};
pub use services::rate_windows::{AgentAction, RateWindows};
pub use services::report_rules::{
    REPORT_MAX_CHARS, ReportKind, ReportReceiver, report_line, report_receiver, report_record_text,
    validate_report_text,
};
pub use services::tree_rules::{
    Placement, TreeRow, check_placement, descendants_in_tree_order, descendants_of,
    is_descendant_of, resolved_parent,
};
pub use services::visible_text::visible_text;
pub use value_objects::{Role, TeamName, parse_role};
