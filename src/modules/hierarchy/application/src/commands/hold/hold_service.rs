// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use hierarchy_domain::{
    AgentAction, AutoHoldTracker, Hold, HoldScope, HoldSetter, LimitRefusal, RateWindows, TreeRow,
    hold_refusal,
};
use kernel::AgentId;

use crate::common::errors::HierarchyApplicationError;
use crate::common::interfaces::HoldStore;
use crate::common::ports::{HoldClock, HoldJournal, HoldNotifier, RateLimitSource};

const OFFICE_ALREADY_HELD: &str = "the office is already on hold";
const AGENT_ALREADY_HELD: &str = "the agent is already on hold";
const OFFICE_NOT_HELD: &str = "the office is not on hold";
const AGENT_NOT_HELD: &str = "the agent is not on hold";

/// The holds as the hold panel shows them: the office hold, if any, and the
/// agent holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HoldsView {
    pub office: Option<Hold>,
    pub agents: Vec<Hold>,
}

/// Why `guard` refused an agent action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardRefusal {
    /// The text the caller answers with.
    pub reason: String,
    /// Whether this refusal made the office hold the caller, so the host
    /// publishes `hold.changed`.
    pub holds_changed: bool,
}

struct HoldState {
    holds: Vec<Hold>,
    windows: RateWindows,
    tracker: AutoHoldTracker,
}

/// What `decide` found under the lock; the ports run after it is released,
/// so a port that calls back into the service cannot wait on itself.
struct Refused {
    reason: String,
    auto_hold: Option<(Hold, LimitRefusal)>,
}

/// Owns the holds, the rate windows, and the automatic-hold tracker behind
/// one mutex, so the hold check, the rate count, and an automatic hold are
/// one step (spec A10).
pub struct HoldService {
    store: Arc<dyn HoldStore>,
    limits: Arc<dyn RateLimitSource>,
    journal: Arc<dyn HoldJournal>,
    notifier: Arc<dyn HoldNotifier>,
    clock: Arc<dyn HoldClock>,
    state: Mutex<HoldState>,
}

impl HoldService {
    /// Loads the stored holds.
    ///
    /// # Errors
    /// `Storage` when the holds cannot be read.
    pub fn new(
        store: Arc<dyn HoldStore>,
        limits: Arc<dyn RateLimitSource>,
        journal: Arc<dyn HoldJournal>,
        notifier: Arc<dyn HoldNotifier>,
        clock: Arc<dyn HoldClock>,
    ) -> Result<Self, HierarchyApplicationError> {
        let holds = store.load_holds()?;
        Ok(Self {
            store,
            limits,
            journal,
            notifier,
            clock,
            state: Mutex::new(HoldState {
                holds,
                windows: RateWindows::default(),
                tracker: AutoHoldTracker::default(),
            }),
        })
    }

    /// A poisoned lock is reused: a panic cannot leave a half-updated list,
    /// because memory changes only after the store write succeeded.
    fn locked(&self) -> MutexGuard<'_, HoldState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[must_use]
    pub fn list(&self) -> HoldsView {
        view_of(&self.locked().holds)
    }

    /// Puts a hold on the office or one agent, set by the user. Returns the
    /// holds as they are now, for `hold.changed`.
    ///
    /// # Errors
    /// `Conflict` when the scope is already held; `Storage` when the write
    /// fails, in which case the previous holds stay.
    pub fn set_hold(&self, scope: HoldScope) -> Result<HoldsView, HierarchyApplicationError> {
        let hold = Hold {
            scope,
            set_by: HoldSetter::User,
            since: self.clock.now(),
            reason: None,
        };
        let view = {
            let mut state = self.locked();
            if is_held(&state.holds, scope) {
                return Err(HierarchyApplicationError::Conflict(
                    match scope {
                        HoldScope::Office => OFFICE_ALREADY_HELD,
                        HoldScope::Agent(_) => AGENT_ALREADY_HELD,
                    }
                    .to_owned(),
                ));
            }
            self.store.save_hold(&hold)?;
            state.holds.push(hold.clone());
            view_of(&state.holds)
        };
        self.journal.held(&hold);
        Ok(view)
    }

    /// Lifts one hold. Lifting the office hold leaves the agent holds.
    ///
    /// # Errors
    /// `Conflict` when the scope is not held; `Storage` when the write
    /// fails, in which case the previous holds stay.
    pub fn release_hold(&self, scope: HoldScope) -> Result<HoldsView, HierarchyApplicationError> {
        let view = {
            let mut state = self.locked();
            if !is_held(&state.holds, scope) {
                return Err(HierarchyApplicationError::Conflict(
                    match scope {
                        HoldScope::Office => OFFICE_NOT_HELD,
                        HoldScope::Agent(_) => AGENT_NOT_HELD,
                    }
                    .to_owned(),
                ));
            }
            self.store.delete_hold(scope)?;
            state.holds.retain(|hold| hold.scope != scope);
            view_of(&state.holds)
        };
        self.journal.released(scope);
        Ok(view)
    }

    /// Checks one agent action in the order of spec A10, after the caller
    /// passed the call rules: holds first (a hold refusal feeds neither the
    /// rate windows nor the automatic-hold tracker), then the rate limit.
    /// A passed action counts toward its window (D2). The third limit
    /// refusal inside one window holds the caller automatically.
    ///
    /// # Errors
    /// A `GuardRefusal` with the reason text; every refusal is journaled.
    pub fn guard(
        &self,
        caller: AgentId,
        action: AgentAction,
        target: Option<AgentId>,
        rows: &[TreeRow],
    ) -> Result<(), GuardRefusal> {
        let Err(refused) = self.decide(caller, action, target, rows) else {
            return Ok(());
        };
        self.journal.refused(caller, &refused.reason);
        let holds_changed = refused.auto_hold.is_some();
        if let Some((hold, limit)) = &refused.auto_hold {
            self.journal.held(hold);
            self.notifier.auto_held(caller, limit);
        }
        Err(GuardRefusal {
            reason: refused.reason,
            holds_changed,
        })
    }

    fn decide(
        &self,
        caller: AgentId,
        action: AgentAction,
        target: Option<AgentId>,
        rows: &[TreeRow],
    ) -> Result<(), Refused> {
        let mut state = self.locked();
        if let Some(refusal) = hold_refusal(&state.holds, rows, caller, target) {
            return Err(Refused {
                reason: refusal.to_string(),
                auto_hold: None,
            });
        }
        let now = self.clock.now();
        // Read per call, so a changed limit applies to the next action.
        let limits = self.limits.limits();
        let Err(limit) = state.windows.try_take(caller, action, &limits, now) else {
            return Ok(());
        };
        let should_hold = state.tracker.record_limit_refusal(caller, now)
            && !is_held(&state.holds, HoldScope::Agent(caller));
        let auto_hold = should_hold
            .then(|| self.store_automatic_hold(&mut state, caller, limit))
            .flatten()
            .map(|hold| (hold, limit));
        Err(Refused {
            reason: limit.to_string(),
            auto_hold,
        })
    }

    /// A store failure keeps the refusal and leaves the agent unheld; the
    /// next refusals count again toward a new automatic hold.
    fn store_automatic_hold(
        &self,
        state: &mut HoldState,
        caller: AgentId,
        limit: LimitRefusal,
    ) -> Option<Hold> {
        let hold = Hold {
            scope: HoldScope::Agent(caller),
            set_by: HoldSetter::Office,
            since: self.clock.now(),
            reason: Some(limit.to_string()),
        };
        self.store.save_hold(&hold).ok()?;
        state.holds.push(hold.clone());
        Some(hold)
    }

    /// Drops the holds of agents that were deleted.
    ///
    /// # Errors
    /// `Storage` when the write fails, in which case the holds stay.
    pub fn forget_agents(&self, agent_ids: &[AgentId]) -> Result<(), HierarchyApplicationError> {
        let mut state = self.locked();
        let gone: HashSet<AgentId> = agent_ids.iter().copied().collect();
        let doomed: Vec<AgentId> = held_agents(&state.holds)
            .filter(|agent_id| gone.contains(agent_id))
            .collect();
        if doomed.is_empty() {
            return Ok(());
        }
        self.store.delete_agent_holds(&doomed)?;
        state.holds.retain(|hold| match hold.scope {
            HoldScope::Agent(agent_id) => !gone.contains(&agent_id),
            HoldScope::Office => true,
        });
        Ok(())
    }

    /// At startup, drops the holds of agents that no longer exist.
    ///
    /// # Errors
    /// `Storage` when the write fails, in which case the holds stay.
    pub fn prune(&self, existing: &HashSet<AgentId>) -> Result<(), HierarchyApplicationError> {
        let missing: Vec<AgentId> = held_agents(&self.locked().holds)
            .filter(|agent_id| !existing.contains(agent_id))
            .collect();
        self.forget_agents(&missing)
    }
}

fn is_held(holds: &[Hold], scope: HoldScope) -> bool {
    holds.iter().any(|hold| hold.scope == scope)
}

fn held_agents(holds: &[Hold]) -> impl Iterator<Item = AgentId> + '_ {
    holds.iter().filter_map(|hold| match hold.scope {
        HoldScope::Agent(agent_id) => Some(agent_id),
        HoldScope::Office => None,
    })
}

fn view_of(holds: &[Hold]) -> HoldsView {
    HoldsView {
        office: holds
            .iter()
            .find(|hold| hold.scope == HoldScope::Office)
            .cloned(),
        agents: holds
            .iter()
            .filter(|hold| hold.scope != HoldScope::Office)
            .cloned()
            .collect(),
    }
}

#[cfg(test)]
#[path = "hold_service.tests.rs"]
mod tests;
