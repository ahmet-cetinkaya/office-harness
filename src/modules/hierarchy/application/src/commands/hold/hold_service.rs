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

impl HoldsView {
    /// Splits a stored list into the office hold and the agent holds.
    #[must_use]
    pub fn from_holds(holds: &[Hold]) -> Self {
        Self {
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
}

/// Why `guard` refused an agent action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardRefusal {
    /// The text the caller answers with.
    pub reason: String,
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
    /// Why an automatic hold that was due could not be stored.
    storage_failure: Option<String>,
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
        HoldsView::from_holds(&self.locked().holds)
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
            HoldsView::from_holds(&state.holds)
        };
        self.journal.held(&hold);
        Ok(view)
    }

    /// Holds one agent for the office, after a resume that failed twice
    /// (0013). Returns the new hold, or `None` when the agent is already held:
    /// the user's own hold stays and nothing is announced twice.
    ///
    /// # Errors
    /// `Storage` when the write fails, in which case the previous holds stay.
    pub fn hold_by_office(
        &self,
        agent: AgentId,
        reason: &str,
    ) -> Result<Option<Hold>, HierarchyApplicationError> {
        let scope = HoldScope::Agent(agent);
        let hold = Hold {
            scope,
            set_by: HoldSetter::Office,
            since: self.clock.now(),
            reason: Some(reason.to_owned()),
        };
        {
            let mut state = self.locked();
            if is_held(&state.holds, scope) {
                return Ok(None);
            }
            self.store.save_hold(&hold)?;
            state.holds.push(hold.clone());
        }
        self.journal.held(&hold);
        self.notifier.resume_held(agent, reason);
        Ok(Some(hold))
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
            HoldsView::from_holds(&state.holds)
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
    /// A `GuardRefusal` with the reason text. The caller journals the
    /// refusal; only the automatic hold is journaled here.
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
        if let Some(message) = &refused.storage_failure {
            self.journal.storage_failed(message);
        }
        if let Some((hold, limit)) = &refused.auto_hold {
            self.journal.held(hold);
            self.notifier.auto_held(caller, limit);
        }
        Err(GuardRefusal {
            reason: refused.reason,
        })
    }

    /// The reason a hold would refuse `caller` acting on `target` now, or
    /// `None`. It counts nothing, so a directive that waited can be checked
    /// again before it is typed without using a rate-window slot (0012/FR-005).
    #[must_use]
    pub fn refusal(&self, caller: AgentId, target: AgentId, rows: &[TreeRow]) -> Option<String> {
        hold_refusal(&self.locked().holds, rows, caller, Some(target))
            .map(|refusal| refusal.to_string())
    }

    /// The reason a hold would refuse a schedule run on `target` now, or
    /// `None`: the office hold, or a hold on the target or an ancestor. It
    /// counts nothing, so the run start and the recheck of a waiting
    /// scheduled directive can ask without using a slot of the schedule
    /// window.
    #[must_use]
    pub fn scheduled_refusal(&self, target: AgentId, rows: &[TreeRow]) -> Option<String> {
        hold_refusal(&self.locked().holds, rows, target, None).map(|refusal| refusal.to_string())
    }

    /// Checks one schedule-made action right before it enters the target's
    /// queue (0025/FR-007): holds first, then, for a directive, the schedule
    /// window of the target. A passed directive counts only in that window,
    /// never in an agent's window. A refusal feeds no automatic hold and
    /// starts none: a schedule is no agent, so only the caller journals it.
    ///
    /// # Errors
    /// A `GuardRefusal` with the hold reason, or the limit and its value.
    pub fn guard_scheduled(
        &self,
        action: AgentAction,
        target: AgentId,
        rows: &[TreeRow],
    ) -> Result<(), GuardRefusal> {
        let mut state = self.locked();
        if let Some(refusal) = hold_refusal(&state.holds, rows, target, None) {
            return Err(GuardRefusal {
                reason: refusal.to_string(),
            });
        }
        if !matches!(action, AgentAction::Directive { .. }) {
            return Ok(());
        }
        let limits = self.limits.limits();
        let now = self.clock.steady_now();
        state
            .windows
            .try_take_scheduled(target, &limits, now)
            .map_err(|limit| GuardRefusal {
                reason: limit.to_string(),
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
                storage_failure: None,
            });
        }
        let now = self.clock.steady_now();
        // Read per call, so a changed limit applies to the next action.
        let limits = self.limits.limits();
        let Err(limit) = state.windows.try_take(caller, action, &limits, now) else {
            return Ok(());
        };
        let should_hold = state.tracker.record_limit_refusal(caller, now)
            && !is_held(&state.holds, HoldScope::Agent(caller));
        let mut refused = Refused {
            reason: limit.to_string(),
            auto_hold: None,
            storage_failure: None,
        };
        if should_hold {
            match self.store_automatic_hold(&mut state, caller, limit) {
                Ok(hold) => refused.auto_hold = Some((hold, limit)),
                Err(message) => refused.storage_failure = Some(message),
            }
        }
        Err(refused)
    }

    /// A store failure keeps the refusal and leaves the agent unheld; the
    /// tracker keeps its history, so the next limit refusal tries again.
    fn store_automatic_hold(
        &self,
        state: &mut HoldState,
        caller: AgentId,
        limit: LimitRefusal,
    ) -> Result<Hold, String> {
        let hold = Hold {
            scope: HoldScope::Agent(caller),
            set_by: HoldSetter::Office,
            since: self.clock.now(),
            reason: Some(limit.to_string()),
        };
        self.store
            .save_hold(&hold)
            .map_err(|error| error.to_string())?;
        state.holds.push(hold.clone());
        state.tracker.clear(caller);
        Ok(hold)
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

#[cfg(test)]
#[path = "hold_service.tests.rs"]
mod tests;
