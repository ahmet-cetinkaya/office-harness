// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{HashMap, VecDeque};

use kernel::{AgentId, Timestamp};

use super::rate_windows::RATE_WINDOW_MILLIS;

/// Limit refusals within one window that put an agent on hold (FR-008).
pub const AUTO_HOLD_REFUSAL_COUNT: usize = 3;

/// Counts the limit refusals of each agent over the rolling window (A5).
///
/// The caller feeds it limit refusals only. Hold refusals never count (A5,
/// A10): a held agent is refused on every action, so counting those would
/// hold it again and again for a state it is already in.
#[derive(Debug, Default)]
pub struct AutoHoldTracker {
    refusals: HashMap<AgentId, VecDeque<i64>>,
}

impl AutoHoldTracker {
    /// Records one limit refusal; true when it is the third (or later)
    /// within the window. The history stays until `clear`, so a hold that
    /// could not be stored is tried again on the next refusal.
    pub fn record_limit_refusal(&mut self, agent: AgentId, now: Timestamp) -> bool {
        let now_millis = now.as_unix_millis();
        let times = self.refusals.entry(agent).or_default();
        while times
            .front()
            .is_some_and(|&refused| now_millis - refused >= RATE_WINDOW_MILLIS)
        {
            times.pop_front();
        }
        times.push_back(now_millis);
        times.len() >= AUTO_HOLD_REFUSAL_COUNT
    }

    /// Forgets the history of one agent, once its automatic hold is stored,
    /// so the next hold needs three new refusals.
    pub fn clear(&mut self, agent: AgentId) {
        self.refusals.remove(&agent);
    }
}

#[cfg(test)]
#[path = "auto_hold_tracker.tests.rs"]
mod tests;
