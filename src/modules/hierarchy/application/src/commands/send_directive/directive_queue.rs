// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{HashMap, VecDeque};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use kernel::{Actor, AgentId, Timestamp};

/// How many directives may wait for one target (0012/FR-004).
pub const QUEUE_LIMIT: usize = 20;

/// How many of a target's waiting entries may be reports, so directives from
/// above always keep the rest of the queue (0015/FR-010).
pub const REPORT_SHARE: usize = 10;

/// How long a directive may wait before it is withdrawn (0012/FR-005).
pub const WAIT_LIMIT: Duration = Duration::from_mins(10);

/// The least time between two deliveries to one target, so the target's
/// status has left `waiting for input` for the previous one before the gate
/// reads it again (0012/FR-001, plan D8).
pub const SETTLE_AFTER_TYPING: Duration = Duration::from_secs(2);

/// Why an entry waits, which decides how it is recorded and when the tree
/// still allows it (0015/FR-004, 0020/FR-001, FR-007).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// A directive from a sender to a descendant.
    Directive,
    /// A report to a parent; `text` is what its record entries show.
    Report { text: String },
    /// The objective of a temporary agent, queued by its creator for the
    /// temp itself.
    Objective { by: Actor },
    /// The line a parent reads when its temp ended; `text` is the record
    /// text.
    EndNotice { text: String },
}

impl Origin {
    /// The record text of a report or an end notice, which share the
    /// report share of a queue and the one place per sender.
    #[must_use]
    pub fn report_text(&self) -> Option<&str> {
        match self {
            Self::Report { text } | Self::EndNotice { text } => Some(text),
            Self::Directive | Self::Objective { .. } => None,
        }
    }
}

/// A directive the office accepted and has not typed yet. A report to a
/// parent waits the same way (0015/FR-004): its `text` is the line to type,
/// and `origin` holds the text its record entries show. `from_temp` says the
/// sender was a temporary agent when the entry was pushed, so the entry
/// outlives the sender's removal (0020/FR-007).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingDirective {
    pub sender: AgentId,
    pub target: AgentId,
    pub text: String,
    pub accepted_at: Timestamp,
    pub origin: Origin,
    pub from_temp: bool,
}

impl WaitingDirective {
    fn is_report_of(&self, sender: AgentId) -> bool {
        self.origin.report_text().is_some() && self.sender == sender
    }
}

/// Where a report may go in its parent's queue (0015/FR-010).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportRoom {
    /// The sender's waiting report is replaced in place.
    Replace,
    /// A new entry at the end.
    Add,
    /// `REPORT_SHARE` reports wait already.
    NoRoomForReports,
    /// `QUEUE_LIMIT` entries wait already.
    Full,
}

/// What became of a report put in its parent's queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportPush {
    /// It took the place of the sender's waiting report, which is returned.
    Replaced {
        position: usize,
        replaced: WaitingDirective,
    },
    Added {
        position: usize,
    },
    Refused(ReportRoom),
    Closed,
}

/// What a delivery round found at the head of one target's queue.
#[derive(Debug, Default)]
pub struct HeadClaim {
    /// Heads that waited past the limit, taken out of the queue.
    pub expired: Vec<WaitingDirective>,
    /// The head the caller now owns the delivery of, still in the queue; the
    /// caller ends the claim with `take_head`, `finish`, or `release`.
    pub head: Option<WaitingDirective>,
}

#[derive(Default)]
struct TargetQueue {
    waiting: VecDeque<WaitingDirective>,
    /// One delivery per target at a time, so its writes and receipts keep
    /// their order (0011 D2) without one lock across every target.
    delivering: bool,
    last_typed: Option<Timestamp>,
}

impl TargetQueue {
    /// The sender's waiting report that a newer one may replace: never the
    /// head whose delivery is claimed, which a delivery may be typing
    /// (0015 review r2 N1).
    fn replaceable_report_of(&self, sender: AgentId) -> Option<usize> {
        let first_free = usize::from(self.delivering);
        self.waiting
            .iter()
            .enumerate()
            .skip(first_free)
            .find(|(_, waiting)| waiting.is_report_of(sender))
            .map(|(index, _)| index)
    }

    fn report_room(&self, sender: AgentId) -> ReportRoom {
        if self.replaceable_report_of(sender).is_some() {
            return ReportRoom::Replace;
        }
        if self.waiting.len() >= QUEUE_LIMIT {
            return ReportRoom::Full;
        }
        let reports = self
            .waiting
            .iter()
            .filter(|waiting| waiting.origin.report_text().is_some())
            .count();
        if reports >= REPORT_SHARE {
            return ReportRoom::NoRoomForReports;
        }
        ReportRoom::Add
    }

    fn settled(&self, now: Timestamp) -> bool {
        self.last_typed
            .is_none_or(|typed| at_least(typed, now, SETTLE_AFTER_TYPING))
    }
}

/// The queue of every target, and whether a server stop closed them all.
#[derive(Default)]
struct Queues {
    by_target: HashMap<AgentId, TargetQueue>,
    /// Set by `close`: no directive enters after it, so none waits without
    /// an outcome (0012/FR-005).
    closed: bool,
}

/// The waiting directives of every target, oldest first per target. It lives
/// in memory only (spec A4).
#[derive(Default)]
pub struct DirectiveQueue {
    queues: Mutex<Queues>,
    work: Condvar,
}

impl DirectiveQueue {
    fn locked(&self) -> MutexGuard<'_, Queues> {
        self.queues.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[must_use]
    pub fn waiting_count(&self, target: AgentId) -> usize {
        self.locked()
            .by_target
            .get(&target)
            .map_or(0, |queue| queue.waiting.len())
    }

    /// Claims the delivery to `target` for a directive that has not entered
    /// the queue: only when nothing waits or is being delivered there and
    /// the settle gap has passed, so a new directive never overtakes; never
    /// once the queue is closed.
    #[must_use]
    pub fn claim_empty(&self, target: AgentId, now: Timestamp) -> bool {
        let mut queues = self.locked();
        if queues.closed {
            return false;
        }
        let queue = queues.by_target.entry(target).or_default();
        let free = queue.waiting.is_empty() && !queue.delivering && queue.settled(now);
        if free {
            queue.delivering = true;
        }
        free
    }

    /// Adds a directive at the end of its target's queue and returns its
    /// place, counted from 1; `None` once the queue is closed.
    pub fn push(&self, directive: WaitingDirective) -> Option<usize> {
        let mut queues = self.locked();
        if queues.closed {
            return None;
        }
        let queue = queues.by_target.entry(directive.target).or_default();
        queue.waiting.push_back(directive);
        let position = queue.waiting.len();
        drop(queues);
        self.work.notify_all();
        Some(position)
    }

    /// Where a report of `sender` would go in `target`'s queue now.
    #[must_use]
    pub fn report_room(&self, target: AgentId, sender: AgentId) -> ReportRoom {
        self.locked()
            .by_target
            .get(&target)
            .map_or(ReportRoom::Add, |queue| queue.report_room(sender))
    }

    /// Puts a report in its parent's queue under one lock: it replaces the
    /// sender's waiting report in place, or joins the end when the queue and
    /// the report share have room (0015/FR-010). The replacement keeps the
    /// place but its own acceptance time, so the wait limit counts from it
    /// (0015 review r2 N2).
    pub fn push_report(&self, report: WaitingDirective) -> ReportPush {
        let mut queues = self.locked();
        if queues.closed {
            return ReportPush::Closed;
        }
        let queue = queues.by_target.entry(report.target).or_default();
        let pushed = match queue.report_room(report.sender) {
            ReportRoom::Replace => {
                let index = queue
                    .replaceable_report_of(report.sender)
                    .unwrap_or_default();
                let replaced = std::mem::replace(&mut queue.waiting[index], report);
                ReportPush::Replaced {
                    position: index + 1,
                    replaced,
                }
            }
            ReportRoom::Add => {
                queue.waiting.push_back(report);
                ReportPush::Added {
                    position: queue.waiting.len(),
                }
            }
            refused => ReportPush::Refused(refused),
        };
        drop(queues);
        self.work.notify_all();
        pushed
    }

    /// Takes the expired heads out and, when the target is free and settled,
    /// claims the delivery of the next head. A closed queue holds no head.
    pub fn claim_head(&self, target: AgentId, now: Timestamp) -> HeadClaim {
        let mut queues = self.locked();
        let Some(queue) = queues.by_target.get_mut(&target) else {
            return HeadClaim::default();
        };
        let mut claim = HeadClaim::default();
        while queue
            .waiting
            .front()
            .is_some_and(|head| at_least(head.accepted_at, now, WAIT_LIMIT))
        {
            claim.expired.extend(queue.waiting.pop_front());
        }
        if !queue.delivering && queue.settled(now) {
            claim.head = queue.waiting.front().cloned();
            queue.delivering = claim.head.is_some();
        }
        claim
    }

    /// Takes the claimed head out of the queue and says whether it was
    /// still there: a server stop may have withdrawn it since the claim. The
    /// claim stays until `finish` or `release`.
    #[must_use]
    pub fn take_head(&self, claimed: &WaitingDirective) -> bool {
        self.locked()
            .by_target
            .get_mut(&claimed.target)
            .and_then(|queue| queue.waiting.pop_front_if(|head| head == claimed))
            .is_some()
    }

    /// Ends a claim after a delivery: the next one waits the settle gap.
    pub fn finish(&self, target: AgentId, now: Timestamp) {
        if let Some(queue) = self.locked().by_target.get_mut(&target) {
            queue.delivering = false;
            queue.last_typed = Some(now);
        }
    }

    /// Ends a claim that typed nothing.
    pub fn release(&self, target: AgentId) {
        if let Some(queue) = self.locked().by_target.get_mut(&target) {
            queue.delivering = false;
        }
    }

    /// Takes every waiting directive of one target out of the queue.
    pub fn drain(&self, target: AgentId) -> Vec<WaitingDirective> {
        self.locked()
            .by_target
            .get_mut(&target)
            .map_or_else(Vec::new, |queue| queue.waiting.drain(..).collect())
    }

    /// Closes the queue for a server stop and takes every waiting directive
    /// of every target out: from now on nothing enters or is claimed.
    pub fn close(&self) -> Vec<WaitingDirective> {
        let mut queues = self.locked();
        queues.closed = true;
        queues
            .by_target
            .values_mut()
            .flat_map(|queue| queue.waiting.drain(..))
            .collect()
    }

    /// The targets that have directives waiting.
    #[must_use]
    pub fn targets_with_waiting(&self) -> Vec<AgentId> {
        self.locked()
            .by_target
            .iter()
            .filter(|(_, queue)| !queue.waiting.is_empty())
            .map(|(target, _)| *target)
            .collect()
    }

    /// Blocks until a directive waits or `timeout` passes, and says whether
    /// one waits.
    pub fn wait_for_work(&self, timeout: Duration) -> bool {
        let queues = self.locked();
        let (queues, _) = self
            .work
            .wait_timeout_while(queues, timeout, |queues| {
                queues
                    .by_target
                    .values()
                    .all(|queue| queue.waiting.is_empty())
            })
            .unwrap_or_else(PoisonError::into_inner);
        queues
            .by_target
            .values()
            .any(|queue| !queue.waiting.is_empty())
    }
}

/// True when at least `span` lies between `earlier` and `now`.
fn at_least(earlier: Timestamp, now: Timestamp, span: Duration) -> bool {
    let span_millis = i64::try_from(span.as_millis()).unwrap_or(i64::MAX);
    now.as_unix_millis()
        .saturating_sub(earlier.as_unix_millis())
        >= span_millis
}

#[cfg(test)]
#[path = "directive_queue.tests.rs"]
mod tests;
