// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::{AgentId, Timestamp};

use crate::value_objects::{AttentionEvent, AttentionKind, NotificationId};

const FAILED_GROUP: u8 = 0;
const WAITING_GROUP: u8 = 1;
const EXITED_GROUP: u8 = 2;

/// One agent that waits for the user. There is one item per agent: a second
/// event changes `latest` and leaves the wait time alone (0008 D1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeedsYouItem {
    pub agent_id: AgentId,
    /// When the agent began to wait; the longest wait comes first.
    pub first_event_at: Timestamp,
    pub latest: AttentionEvent,
    /// The notification that created the item, if one was created. A read of
    /// it clears the item.
    pub origin: Option<NotificationId>,
    /// The agent's latest report for the person, kept through later events
    /// until the item leaves (0015/FR-008).
    pub report: Option<ItemReport>,
}

/// A report for the person as its item shows it: the kind's wire name and
/// the text in visible form, as the office accepted them (0015/FR-003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemReport {
    pub kind: String,
    pub text: String,
    pub at: Timestamp,
}

impl NeedsYouItem {
    const fn group_rank(&self) -> u8 {
        match self.latest.kind {
            AttentionKind::Failed => FAILED_GROUP,
            AttentionKind::WaitingForInput
            | AttentionKind::Bell
            | AttentionKind::AutoHold
            | AttentionKind::ResumeHold
            | AttentionKind::LeftoverKept
            | AttentionKind::Report => WAITING_GROUP,
            AttentionKind::Exited => EXITED_GROUP,
        }
    }
}

/// The queue order: failed agents, then agents that wait, ring, or were held,
/// then exited agents. Inside a group the longest wait comes first, then the
/// agent name.
#[must_use]
pub fn order(
    mut items: Vec<NeedsYouItem>,
    agent_name: impl Fn(AgentId) -> String,
) -> Vec<NeedsYouItem> {
    items.sort_by_cached_key(|item| {
        (
            item.group_rank(),
            item.first_event_at,
            agent_name(item.agent_id),
        )
    });
    items
}

/// The item after an event: a new one when the agent has none, else the old
/// one with the new latest event. The old wait time, origin, and report
/// stay.
#[must_use]
pub fn record_event(
    existing: Option<NeedsYouItem>,
    agent_id: AgentId,
    event: AttentionEvent,
    origin: Option<NotificationId>,
) -> NeedsYouItem {
    match existing {
        Some(item) => NeedsYouItem {
            latest: event,
            ..item
        },
        None => NeedsYouItem {
            agent_id,
            first_event_at: event.at,
            latest: event,
            origin,
            report: None,
        },
    }
}

/// The item after a report for the person: the report is its latest event
/// and replaces any earlier report; the wait time and origin stay as
/// `record_event` keeps them (0015/FR-008).
#[must_use]
pub fn record_report(
    existing: Option<NeedsYouItem>,
    agent_id: AgentId,
    report: ItemReport,
    origin: Option<NotificationId>,
) -> NeedsYouItem {
    let event = AttentionEvent {
        kind: AttentionKind::Report,
        detail: Some(report.kind.clone()),
        at: report.at,
    };
    NeedsYouItem {
        report: Some(report),
        ..record_event(existing, agent_id, event, origin)
    }
}
