// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{HashMap, VecDeque};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use kernel::{AgentId, Timestamp};

/// How many directives may wait for one target (0012/FR-004).
pub const QUEUE_LIMIT: usize = 20;

/// How long a directive may wait before it is withdrawn (0012/FR-005).
pub const WAIT_LIMIT: Duration = Duration::from_mins(10);

/// The least time between two deliveries to one target, so the target's
/// status has left `waiting for input` for the previous one before the gate
/// reads it again (0012/FR-001, plan D8).
pub const SETTLE_AFTER_TYPING: Duration = Duration::from_secs(2);

/// A directive the office accepted and has not typed yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingDirective {
    pub sender: AgentId,
    pub target: AgentId,
    pub text: String,
    pub accepted_at: Timestamp,
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
    fn settled(&self, now: Timestamp) -> bool {
        self.last_typed
            .is_none_or(|typed| at_least(typed, now, SETTLE_AFTER_TYPING))
    }
}

/// The waiting directives of every target, oldest first per target. It lives
/// in memory only (spec A4).
#[derive(Default)]
pub struct DirectiveQueue {
    targets: Mutex<HashMap<AgentId, TargetQueue>>,
    work: Condvar,
}

impl DirectiveQueue {
    fn locked(&self) -> MutexGuard<'_, HashMap<AgentId, TargetQueue>> {
        self.targets.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[must_use]
    pub fn waiting_count(&self, target: AgentId) -> usize {
        self.locked()
            .get(&target)
            .map_or(0, |queue| queue.waiting.len())
    }

    /// Claims the delivery to `target` for a directive that has not entered
    /// the queue: only when nothing waits or is being delivered there and
    /// the settle gap has passed, so a new directive never overtakes.
    #[must_use]
    pub fn claim_empty(&self, target: AgentId, now: Timestamp) -> bool {
        let mut targets = self.locked();
        let queue = targets.entry(target).or_default();
        let free = queue.waiting.is_empty() && !queue.delivering && queue.settled(now);
        if free {
            queue.delivering = true;
        }
        free
    }

    /// Adds a directive at the end of its target's queue and returns its
    /// place, counted from 1.
    pub fn push(&self, directive: WaitingDirective) -> usize {
        let mut targets = self.locked();
        let queue = targets.entry(directive.target).or_default();
        queue.waiting.push_back(directive);
        let position = queue.waiting.len();
        drop(targets);
        self.work.notify_all();
        position
    }

    /// Takes the expired heads out and, when the target is free and settled,
    /// claims the delivery of the next head.
    pub fn claim_head(&self, target: AgentId, now: Timestamp) -> HeadClaim {
        let mut targets = self.locked();
        let Some(queue) = targets.get_mut(&target) else {
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

    /// Takes the claimed head out of the queue; the claim stays until
    /// `finish` or `release`.
    pub fn take_head(&self, target: AgentId) -> Option<WaitingDirective> {
        self.locked()
            .get_mut(&target)
            .and_then(|queue| queue.waiting.pop_front())
    }

    /// Ends a claim after a delivery: the next one waits the settle gap.
    pub fn finish(&self, target: AgentId, now: Timestamp) {
        if let Some(queue) = self.locked().get_mut(&target) {
            queue.delivering = false;
            queue.last_typed = Some(now);
        }
    }

    /// Ends a claim that typed nothing.
    pub fn release(&self, target: AgentId) {
        if let Some(queue) = self.locked().get_mut(&target) {
            queue.delivering = false;
        }
    }

    /// Takes every waiting directive of one target out of the queue.
    pub fn drain(&self, target: AgentId) -> Vec<WaitingDirective> {
        self.locked()
            .get_mut(&target)
            .map_or_else(Vec::new, |queue| queue.waiting.drain(..).collect())
    }

    /// Takes every waiting directive of every target out of the queue.
    pub fn drain_all(&self) -> Vec<WaitingDirective> {
        self.locked()
            .values_mut()
            .flat_map(|queue| queue.waiting.drain(..))
            .collect()
    }

    /// The targets that have directives waiting.
    #[must_use]
    pub fn targets_with_waiting(&self) -> Vec<AgentId> {
        self.locked()
            .iter()
            .filter(|(_, queue)| !queue.waiting.is_empty())
            .map(|(target, _)| *target)
            .collect()
    }

    /// Blocks until a directive waits or `timeout` passes, and says whether
    /// one waits.
    pub fn wait_for_work(&self, timeout: Duration) -> bool {
        let targets = self.locked();
        let (targets, _) = self
            .work
            .wait_timeout_while(targets, timeout, |targets| {
                targets.values().all(|queue| queue.waiting.is_empty())
            })
            .unwrap_or_else(PoisonError::into_inner);
        targets.values().any(|queue| !queue.waiting.is_empty())
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
