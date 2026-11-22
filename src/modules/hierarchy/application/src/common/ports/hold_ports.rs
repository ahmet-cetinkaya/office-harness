// SPDX-License-Identifier: AGPL-3.0-or-later

use hierarchy_domain::{Hold, HoldScope, LimitRefusal, RateLimits};
use kernel::{AgentId, Timestamp};

/// The rate limits as the settings hold them now. The guard reads them on
/// every call, so a changed limit applies to the next action (FR-009).
pub trait RateLimitSource: Send + Sync {
    #[must_use]
    fn limits(&self) -> RateLimits;
}

/// The two times the hold service runs on.
pub trait HoldClock: Send + Sync {
    /// The wall time: a hold's `since` and its record entry, so they line up
    /// with every other entry of the action record.
    #[must_use]
    fn now(&self) -> Timestamp;
    /// A time that a step of the system clock or a suspend never moves
    /// backward or forward: the rate windows and the automatic-hold tracker
    /// count on it, so a limit always spans 60 real seconds.
    #[must_use]
    fn steady_now(&self) -> Timestamp;
}

/// Keeps hold changes in the action record. The service never names an
/// actor; the host bridge maps each call to one: `held` of a user hold and
/// `released` use the user actor, `held` of an automatic hold (`set_by` is
/// `Office`) uses the office actor. The bridge owns any record-write
/// failure. A refusal is not journaled here: the caller of `guard` records
/// it, because only the caller knows the action, the target, and any text
/// (spec FR-010, 0002/FR-011).
pub trait HoldJournal: Send + Sync {
    fn held(&self, hold: &Hold);
    fn released(&self, scope: HoldScope);
    /// The service could not store an automatic hold; the host tells the
    /// user the disk failed.
    fn storage_failed(&self, message: &str);
}

/// Tells the user that the office held an agent by itself. The service
/// calls it once per hold, after it released its own lock.
pub trait HoldNotifier: Send + Sync {
    fn auto_held(&self, agent: AgentId, limit: &LimitRefusal);
    /// The office held the agent because its resume failed twice (0013).
    fn resume_held(&self, agent: AgentId, reason: &str);
}
