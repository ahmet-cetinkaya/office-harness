// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

/// Length of every rate window, in seconds (A3). The refusal text and the
/// windows both read this one value.
pub const RATE_WINDOW_SECONDS: u32 = 60;

pub const DEFAULT_DIRECTIVE_LIMIT: u32 = 10;
pub const DEFAULT_DIRECTIVE_TARGET_LIMIT: u32 = 3;
pub const DEFAULT_CREATE_LIMIT: u32 = 5;
pub const DEFAULT_LIFECYCLE_LIMIT: u32 = 10;
pub const DEFAULT_REPORT_LIMIT: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LimitKind {
    Directive,
    DirectiveTarget,
    Create,
    Lifecycle,
    Report,
}

impl LimitKind {
    /// The name a refusal shows to the agent.
    #[must_use]
    pub const fn human_name(self) -> &'static str {
        match self {
            Self::Directive => "directives sent by one agent",
            Self::DirectiveTarget => "directives sent to one target agent",
            Self::Create => "agents created by one agent",
            Self::Lifecycle => "starts, stops, and new tasks by one agent",
            Self::Report => "reports sent by one agent",
        }
    }
}

/// The allowed count of each kind per rolling window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimits {
    pub directive: u32,
    pub directive_target: u32,
    pub create: u32,
    pub lifecycle: u32,
    pub report: u32,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            directive: DEFAULT_DIRECTIVE_LIMIT,
            directive_target: DEFAULT_DIRECTIVE_TARGET_LIMIT,
            create: DEFAULT_CREATE_LIMIT,
            lifecycle: DEFAULT_LIFECYCLE_LIMIT,
            report: DEFAULT_REPORT_LIMIT,
        }
    }
}

impl RateLimits {
    #[must_use]
    pub const fn allowed(&self, kind: LimitKind) -> u32 {
        match kind {
            LimitKind::Directive => self.directive,
            LimitKind::DirectiveTarget => self.directive_target,
            LimitKind::Create => self.create,
            LimitKind::Lifecycle => self.lifecycle,
            LimitKind::Report => self.report,
        }
    }
}

/// The reason a rate limit refuses an agent-made action (FR-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LimitRefusal {
    pub kind: LimitKind,
    pub allowed: u32,
}

impl fmt::Display for LimitRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "the limit \"{}\" allows {} per {RATE_WINDOW_SECONDS} seconds",
            self.kind.human_name(),
            self.allowed
        )
    }
}
