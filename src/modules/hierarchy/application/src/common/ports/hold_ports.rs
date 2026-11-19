// SPDX-License-Identifier: AGPL-3.0-or-later

use hierarchy_domain::{Hold, HoldScope, LimitRefusal, RateLimits};
use kernel::{AgentId, Timestamp};

/// The rate limits as the settings hold them now. The guard reads them on
/// every call, so a changed limit applies to the next action (FR-009).
pub trait RateLimitSource: Send + Sync {
    #[must_use]
    fn limits(&self) -> RateLimits;
}

/// The time the holds and the rate windows run on.
pub trait HoldClock: Send + Sync {
    #[must_use]
    fn now(&self) -> Timestamp;
}

/// Keeps hold changes and refusals in the action record. The service never
/// names an actor; the host bridge maps each call to one: `held` of a user
/// hold and `released` use the user actor, `held` of an automatic hold
/// (`set_by` is `Office`) uses the office actor, and `refused` uses the
/// refused caller. The bridge owns any record-write failure.
pub trait HoldJournal: Send + Sync {
    fn held(&self, hold: &Hold);
    fn released(&self, scope: HoldScope);
    fn refused(&self, caller: AgentId, reason: &str);
}

/// Tells the user that the office held an agent by itself. The service
/// calls it once per automatic hold, after it released its own lock.
pub trait HoldNotifier: Send + Sync {
    fn auto_held(&self, agent: AgentId, limit: &LimitRefusal);
}
