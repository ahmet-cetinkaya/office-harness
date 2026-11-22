// SPDX-License-Identifier: AGPL-3.0-or-later

use hierarchy_domain::ReportKind;
use kernel::AgentId;

/// Keeps reports in the action record (0015/FR-009). The bridge owns a
/// failed record write, so a report is never refused for a record failure.
pub trait ReportJournal: Send + Sync {
    /// Records an accepted report under `receiver`: the parent, or the
    /// sender itself for a report for the person. The sender is the actor.
    fn accepted(&self, sender: AgentId, receiver: AgentId, record_text: &str);

    /// Records a refused report, with the text as it was sent, in the
    /// sender's record and, when the report had one, in the parent's.
    fn refused(&self, sender: AgentId, parent: Option<AgentId>, text: &str, reason: &str);
}

/// Hands a report for the person to the owner: a notification and the
/// report on the sender's Needs you item (0015/FR-007). The bridge owns a
/// failed write and raises the office's storage alert, as for every
/// attention event.
pub trait PersonReports: Send + Sync {
    /// `text` is already in visible form.
    fn deliver(&self, sender: AgentId, kind: ReportKind, text: &str);
}
