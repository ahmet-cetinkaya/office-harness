// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod auto_hold_tracker;
pub mod call_rules;
pub mod hold;
pub mod rate_limits;
pub mod rate_windows;
pub mod report_rules;
pub mod tree_rules;
pub mod visible_text;

pub use auto_hold_tracker::{AUTO_HOLD_REFUSAL_COUNT, AutoHoldTracker};
pub use call_rules::{
    DIRECTIVE_MAX_CHARS, check_descendant, creation_parent, validate_directive_text,
};
pub use hold::{Hold, HoldRefusal, HoldScope, HoldSetter, hold_refusal};
pub use rate_limits::{LimitKind, LimitRefusal, RATE_WINDOW_SECONDS, RateLimits};
pub use rate_windows::{AgentAction, RateWindows};
pub use report_rules::{
    REPORT_MAX_CHARS, ReportKind, ReportReceiver, report_line, report_receiver, report_record_text,
    validate_report_text,
};
pub use tree_rules::{
    Placement, TreeRow, check_placement, descendants_in_tree_order, descendants_of,
    is_descendant_of, resolved_parent,
};
pub use visible_text::visible_text;
