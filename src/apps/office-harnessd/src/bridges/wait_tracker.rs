// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use kernel::AgentId;
use terminal_application::common::models::RunId;

/// How much longer than the quiet period a stretch of `working` may last
/// before it counts as work and not as a redraw (0022 A10). A redraw shows
/// `working` for about one quiet period.
pub const WORK_BOUND_EXTRA: Duration = Duration::from_secs(10);

/// The quiet period that the work bound follows until the terminals tell
/// it theirs: the default of the setting (0001).
const DEFAULT_QUIET_PERIOD: Duration = Duration::from_secs(5);

/// A change of an agent's status between the two states that a redraw can
/// toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusChange {
    ToWorking,
    ToWaitingForInput,
}

/// Whether something that the office knows of caused a change (0022 Terms).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Prompted,
    Unprompted,
}

struct AgentWaits {
    /// A wait end happened since the last change to `waiting for input`. A
    /// session starts with one, so its first wait is prompted.
    ended_since_wait: bool,
    working_since: Option<Instant>,
    last_prompted_wait: Option<Instant>,
    run: Option<RunId>,
    /// The session's status follows the reports of its CLI, so every change
    /// is prompted (0024/FR-011).
    exact: bool,
}

impl AgentWaits {
    fn after_session_boundary(run: Option<RunId>) -> Self {
        Self {
            ended_since_wait: true,
            working_since: None,
            last_prompted_wait: None,
            run,
            exact: false,
        }
    }
}

struct Waits {
    agents: HashMap<AgentId, AgentWaits>,
    quiet_period: Duration,
    work_bound_extra: Duration,
}

impl Waits {
    fn work_bound(&self) -> Duration {
        self.quiet_period + self.work_bound_extra
    }

    fn agent(&mut self, agent_id: AgentId) -> &mut AgentWaits {
        self.agents
            .entry(agent_id)
            .or_insert_with(|| AgentWaits::after_session_boundary(None))
    }
}

/// Called with the agent after each wait end, outside the tracker's lock.
type WaitEndListener = Arc<dyn Fn(AgentId) + Send + Sync>;

/// Tells a prompted change of an agent's status from an unprompted one,
/// from the wait ends that the server sees itself and never from the screen
/// (0022 A2). Clones share one record.
#[derive(Clone)]
pub struct WaitTracker {
    waits: Arc<Mutex<Waits>>,
    wait_end_listener: Arc<OnceLock<WaitEndListener>>,
    ask_wait_end_listener: Arc<OnceLock<WaitEndListener>>,
}

impl Default for WaitTracker {
    fn default() -> Self {
        Self::new(DEFAULT_QUIET_PERIOD)
    }
}

impl WaitTracker {
    /// A tracker whose work bound is `quiet_period` plus [`WORK_BOUND_EXTRA`].
    #[must_use]
    pub fn new(quiet_period: Duration) -> Self {
        Self {
            waits: Arc::new(Mutex::new(Waits {
                agents: HashMap::new(),
                quiet_period,
                work_bound_extra: WORK_BOUND_EXTRA,
            })),
            wait_end_listener: Arc::default(),
            ask_wait_end_listener: Arc::default(),
        }
    }

    fn locked(&self) -> MutexGuard<'_, Waits> {
        self.waits.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Replaces the extra time of the work bound; tests shorten it.
    pub fn set_work_bound_extra(&self, work_bound_extra: Duration) {
        self.locked().work_bound_extra = work_bound_extra;
    }

    /// How long a stretch of `working` may last before it counts as work.
    #[must_use]
    pub fn work_bound(&self) -> Duration {
        self.locked().work_bound()
    }

    /// Calls `listener` after each wait end. The first listener stays: the
    /// server has one Needs you queue to tell.
    pub fn on_wait_end(&self, listener: impl Fn(AgentId) + Send + Sync + 'static) {
        let _ = self.wait_end_listener.set(Arc::new(listener));
    }

    /// Calls `listener` after each wait end, as well as the listener of
    /// `on_wait_end`: an input at the terminal ends the permission ask of an
    /// exact session (0024/FR-013). The first listener stays.
    pub fn on_ask_wait_end(&self, listener: impl Fn(AgentId) + Send + Sync + 'static) {
        let _ = self.ask_wait_end_listener.set(Arc::new(listener));
    }

    /// A person's input or the office's text reached the agent's terminal.
    pub fn wait_end(&self, agent_id: AgentId) {
        self.locked().agent(agent_id).ended_since_wait = true;
        if let Some(listener) = self.wait_end_listener.get() {
            listener(agent_id);
        }
        if let Some(listener) = self.ask_wait_end_listener.get() {
            listener(agent_id);
        }
    }

    /// The agent's session is exact or not (0024). While it is exact every
    /// change is prompted. Leaving it ends the wait, so the first change by
    /// the quiet period is prompted too.
    pub fn set_exact(&self, agent_id: AgentId, exact: bool) {
        let mut waits = self.locked();
        let agent = waits.agent(agent_id);
        agent.exact = exact;
        if !exact {
            agent.ended_since_wait = true;
        }
    }

    /// A session of the agent's CLI starts in `run`: nothing of an earlier
    /// session carries over, and the start is a wait end.
    pub fn session_started(&self, agent_id: AgentId, run: Option<RunId>) {
        self.locked()
            .agents
            .insert(agent_id, AgentWaits::after_session_boundary(run));
    }

    /// The agent's session ends for any reason: exit, stop, sleep.
    pub fn session_ended(&self, agent_id: AgentId) {
        self.locked()
            .agents
            .insert(agent_id, AgentWaits::after_session_boundary(None));
    }

    /// Judges one change of the agent's status and records what follows
    /// from it. Ask once per change: a prompted wait uses up the wait end.
    #[must_use]
    pub fn classify(&self, agent_id: AgentId, change: StatusChange) -> Change {
        self.classify_at(agent_id, change, Instant::now())
    }

    fn classify_at(&self, agent_id: AgentId, change: StatusChange, now: Instant) -> Change {
        let mut waits = self.locked();
        let work_bound = waits.work_bound();
        let agent = waits.agent(agent_id);
        match change {
            StatusChange::ToWorking => {
                agent.working_since = Some(now);
                prompted_when(agent.ended_since_wait || agent.exact)
            }
            StatusChange::ToWaitingForInput => {
                let long_work = agent
                    .working_since
                    .is_some_and(|since| now.saturating_duration_since(since) > work_bound);
                let verdict = prompted_when(agent.ended_since_wait || long_work || agent.exact);
                if verdict == Change::Prompted {
                    agent.last_prompted_wait = Some(now);
                    agent.ended_since_wait = false;
                }
                verdict
            }
        }
    }

    /// When the agent's current stretch of `working` began.
    #[must_use]
    pub fn working_since(&self, agent_id: AgentId) -> Option<Instant> {
        self.locked().agents.get(&agent_id)?.working_since
    }

    /// When the agent last changed to `waiting for input` with a prompt.
    #[must_use]
    pub fn last_prompted_wait(&self, agent_id: AgentId) -> Option<Instant> {
        self.locked().agents.get(&agent_id)?.last_prompted_wait
    }

    /// The run of the agent's current session, when it has one.
    #[must_use]
    pub fn run(&self, agent_id: AgentId) -> Option<RunId> {
        self.locked().agents.get(&agent_id)?.run
    }
}

fn prompted_when(is_prompted: bool) -> Change {
    if is_prompted {
        Change::Prompted
    } else {
        Change::Unprompted
    }
}

#[cfg(test)]
#[path = "wait_tracker.tests.rs"]
mod tests;
