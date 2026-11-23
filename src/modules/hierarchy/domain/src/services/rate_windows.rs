// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

use kernel::{AgentId, Timestamp};

use super::rate_limits::{LimitKind, LimitRefusal, RATE_WINDOW_SECONDS, RateLimits};

const MILLIS_PER_SECOND: i64 = 1000;
pub(crate) const RATE_WINDOW_MILLIS: i64 = RATE_WINDOW_SECONDS as i64 * MILLIS_PER_SECOND;

/// An agent-made action the rate limits count (A3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentAction {
    Directive {
        target: AgentId,
    },
    Create,
    Lifecycle,
    Report,
    /// One read of a secret by an agent (0026).
    SecretRead,
}

/// Timestamps of passed actions inside a rolling window, per key. Only
/// passed actions are kept: a refused action takes no slot.
#[derive(Debug)]
struct SlidingWindows<Key: Eq + Hash + Copy> {
    entries: HashMap<Key, VecDeque<i64>>,
}

// Written by hand: a derive would demand `Key: Default`, which ids lack.
impl<Key: Eq + Hash + Copy> Default for SlidingWindows<Key> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<Key: Eq + Hash + Copy> SlidingWindows<Key> {
    fn is_full(&mut self, key: Key, allowed: u32, now_millis: i64) -> bool {
        // A limit of 0 allows nothing, even for a key with no history. The
        // settings refuse 0, so this only guards a caller that skips them.
        if allowed == 0 {
            return true;
        }
        let Some(times) = self.entries.get_mut(&key) else {
            return false;
        };
        while times
            .front()
            .is_some_and(|&taken| now_millis - taken >= RATE_WINDOW_MILLIS)
        {
            times.pop_front();
        }
        if times.is_empty() {
            // Prune the key so memory follows the live agents only.
            self.entries.remove(&key);
            return false;
        }
        times.len() >= allowed as usize
    }

    fn record(&mut self, key: Key, now_millis: i64) {
        self.entries.entry(key).or_default().push_back(now_millis);
    }
}

/// The rolling 60-second windows of every rate limit (A3).
#[derive(Debug, Default)]
pub struct RateWindows {
    directives: SlidingWindows<AgentId>,
    directives_to_target: SlidingWindows<(AgentId, AgentId)>,
    creates: SlidingWindows<AgentId>,
    lifecycles: SlidingWindows<AgentId>,
    reports: SlidingWindows<AgentId>,
    secret_reads: SlidingWindows<AgentId>,
}

impl RateWindows {
    /// Takes one slot in every window the action counts in.
    ///
    /// # Errors
    /// The first full window, in the order of the limit table, as a
    /// `LimitRefusal`; nothing is recorded then.
    pub fn try_take(
        &mut self,
        caller: AgentId,
        action: AgentAction,
        limits: &RateLimits,
        now: Timestamp,
    ) -> Result<(), LimitRefusal> {
        let now_millis = now.as_unix_millis();
        match action {
            AgentAction::Directive { target } => {
                let pair = (caller, target);
                if self
                    .directives
                    .is_full(caller, limits.directive, now_millis)
                {
                    return Err(refusal(LimitKind::Directive, limits));
                }
                if self
                    .directives_to_target
                    .is_full(pair, limits.directive_target, now_millis)
                {
                    return Err(refusal(LimitKind::DirectiveTarget, limits));
                }
                self.directives.record(caller, now_millis);
                self.directives_to_target.record(pair, now_millis);
            }
            AgentAction::Create => {
                if self.creates.is_full(caller, limits.create, now_millis) {
                    return Err(refusal(LimitKind::Create, limits));
                }
                self.creates.record(caller, now_millis);
            }
            AgentAction::Lifecycle => {
                if self
                    .lifecycles
                    .is_full(caller, limits.lifecycle, now_millis)
                {
                    return Err(refusal(LimitKind::Lifecycle, limits));
                }
                self.lifecycles.record(caller, now_millis);
            }
            AgentAction::Report => {
                if self.reports.is_full(caller, limits.report, now_millis) {
                    return Err(refusal(LimitKind::Report, limits));
                }
                self.reports.record(caller, now_millis);
            }
            AgentAction::SecretRead => {
                if self
                    .secret_reads
                    .is_full(caller, limits.secret_read, now_millis)
                {
                    return Err(refusal(LimitKind::SecretRead, limits));
                }
                self.secret_reads.record(caller, now_millis);
            }
        }
        Ok(())
    }
}

fn refusal(kind: LimitKind, limits: &RateLimits) -> LimitRefusal {
    LimitRefusal {
        kind,
        allowed: limits.allowed(kind),
    }
}

#[cfg(test)]
#[path = "rate_windows.tests.rs"]
mod tests;
