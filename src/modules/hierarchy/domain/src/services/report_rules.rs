// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use kernel::AgentId;

use crate::errors::HierarchyError;
use crate::services::tree_rules::TreeRow;

/// The longest report text, counted as the text arrives, before its visible
/// form (0015/FR-001, A2).
pub const REPORT_MAX_CHARS: usize = 1000;

/// What a report says, in the sender's words (0015 A1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReportKind {
    Done,
    Question,
    Blocked,
    Info,
}

impl ReportKind {
    pub const ALL: [Self; 4] = [Self::Done, Self::Question, Self::Blocked, Self::Info];

    /// The wire name, which the heading and the record show as well.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Question => "question",
            Self::Blocked => "blocked",
            Self::Info => "info",
        }
    }

    /// # Errors
    /// `UnknownReportKind` for any text but the four wire names.
    pub fn from_wire(text: &str) -> Result<Self, HierarchyError> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_str() == text)
            .ok_or_else(|| HierarchyError::UnknownReportKind {
                text: text.chars().take(REPORT_KIND_ECHO_CHARS).collect(),
            })
    }
}

/// How much of an unknown kind the refusal repeats: enough to recognise the
/// mistake, too little to flood the record.
const REPORT_KIND_ECHO_CHARS: usize = 16;

impl fmt::Display for ReportKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Who receives an accepted report (0015/FR-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportReceiver {
    Parent(AgentId),
    Person,
}

/// Accepts report text that says something and fits the length bound.
/// Control characters are allowed: the office shows them in visible form
/// instead of refusing them (0015/FR-003, A3).
///
/// # Errors
/// `Empty` for blank text; `TooLong` past `REPORT_MAX_CHARS`.
pub fn validate_report_text(text: &str) -> Result<(), HierarchyError> {
    if text.trim().is_empty() {
        return Err(HierarchyError::Empty { field: "text" });
    }
    if text.chars().count() > REPORT_MAX_CHARS {
        return Err(HierarchyError::TooLong {
            field: "text",
            max: REPORT_MAX_CHARS,
        });
    }
    Ok(())
}

/// The receiver of a report from `sender`, read from the tree as it is now:
/// the sender's parent, or the person when the sender has no parent (the
/// CEO, an unplaced agent) or asks for the person (0015/FR-002).
#[must_use]
pub fn report_receiver(tree: &[TreeRow], sender: AgentId, for_person: bool) -> ReportReceiver {
    if for_person {
        return ReportReceiver::Person;
    }
    tree.iter()
        .find(|row| row.agent_id == sender)
        .and_then(|row| row.parent_agent_id)
        .map_or(ReportReceiver::Person, ReportReceiver::Parent)
}

/// The one line typed into the parent's terminal: the office's heading,
/// then the text (0015 A4). `name` and `text` must already be in visible
/// form, so the line holds no line break.
#[must_use]
pub fn report_line(name: &str, sender: AgentId, kind: ReportKind, text: &str) -> String {
    format!("[report from {name} ({sender}): {kind}] {text}")
}

/// The line a parent reads when its temporary agent has ended (0020/FR-007).
/// `name` must already be in visible form, so the line holds no line break.
#[must_use]
pub fn end_notice_line(name: &str, agent_id: AgentId, reason: &str) -> String {
    format!("[temp ended: {name} ({agent_id}): {reason}]")
}

/// The action record's text of a report (0015 contract, "Action record").
#[must_use]
pub fn report_record_text(kind: ReportKind, text: &str) -> String {
    format!("{kind}: {text}")
}

#[cfg(test)]
#[path = "report_rules.tests.rs"]
mod tests;
