// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use agent_domain::enums::StatusEvent;
use kernel::AgentId;
use terminal_application::common::models::RunId;
use terminal_domain::events::TerminalEvent;

use super::wait_tracker::WaitTracker;
use crate::server::{HookEventKind, HookReport};

/// The frame detail of a change to `waiting for input` that ends a turn
/// (0024/FR-012), and of one that ends with an error.
pub const DETAIL_TURN_STOPPED: &str = "turn_stopped";
pub const DETAIL_TURN_FAILED: &str = "turn_failed";
/// The frame detail of a change to `waiting for input` that asks for
/// permission (0024/FR-013).
pub const DETAIL_PERMISSION_ASKED: &str = "permission_asked";

/// Called after a session's hook state changes, with the new state, or
/// `None` when the session ended. It runs outside the record's lock.
type ChangeListener = Arc<dyn Fn(AgentId, Option<HookState>) + Send + Sync>;

/// Where the status of one session comes from (0024 Terms).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusSource {
    /// No report yet, or no office settings file: the quiet period decides.
    Estimated,
    /// The reports of the CLI decide.
    Exact,
    /// A turn seems stalled: the quiet period decides until the next report.
    Late,
}

/// What the office knows of one session from its reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookState {
    pub source: StatusSource,
    /// Why the session is `estimated`, for the agent's details.
    pub reason: Option<String>,
    pub turn_open: bool,
    pub ask_open: bool,
    pub compacting: bool,
}

impl HookState {
    fn estimated(reason: Option<String>) -> Self {
        Self {
            source: StatusSource::Estimated,
            reason,
            turn_open: false,
            ask_open: false,
            compacting: false,
        }
    }

    /// The agent works while a turn is open or a compaction runs, unless it
    /// waits for the answer to a permission ask (0024/FR-011).
    fn works(&self) -> bool {
        (self.turn_open || self.compacting) && !self.ask_open
    }
}

/// Whether a terminal event reaches the status machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalVerdict {
    Pass,
    Drop,
}

/// What a report asks of the status machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportOutcome {
    pub status_event: StatusEvent,
    pub detail: Option<&'static str>,
    /// The report opened a permission ask.
    pub ask_opened: bool,
}

struct SessionRecord {
    run: Option<RunId>,
    state: HookState,
}

/// The hook state of each running session (0024 D6). Clones share one
/// record. A session of a CLI without hooks has a record that stays
/// `estimated`, so the rules of 0022 apply to it unchanged.
#[derive(Clone, Default)]
pub struct HookSessions {
    sessions: Arc<Mutex<HashMap<AgentId, SessionRecord>>>,
    waits: WaitTracker,
    listener: Arc<OnceLock<ChangeListener>>,
}

impl HookSessions {
    /// Tells `waits` which sessions are exact, so that every change of an
    /// exact session reads as prompted (0024/FR-011).
    #[must_use]
    pub fn new(waits: WaitTracker) -> Self {
        Self {
            sessions: Arc::default(),
            waits,
            listener: Arc::default(),
        }
    }

    /// Calls `listener` after each change of a session's state, so that the
    /// clients hear of it (0024 D9). The first listener stays.
    pub fn on_change(&self, listener: impl Fn(AgentId, Option<HookState>) + Send + Sync + 'static) {
        let _ = self.listener.set(Arc::new(listener));
    }

    fn notify(&self, agent_id: AgentId, state: Option<HookState>) {
        if let Some(listener) = self.listener.get() {
            listener(agent_id, state);
        }
    }

    fn locked(&self) -> MutexGuard<'_, HashMap<AgentId, SessionRecord>> {
        self.sessions.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A session starts in `run`. It is `estimated` until its first report;
    /// `reason` says why it may stay so.
    pub fn session_started(&self, agent_id: AgentId, run: Option<RunId>, reason: Option<String>) {
        let state = HookState::estimated(reason);
        self.locked().insert(
            agent_id,
            SessionRecord {
                run,
                state: state.clone(),
            },
        );
        self.notify(agent_id, Some(state));
    }

    /// The agent's session ends for any reason.
    pub fn session_ended(&self, agent_id: AgentId) {
        let removed = self.locked().remove(&agent_id);
        if removed.is_some() {
            self.notify(agent_id, None);
        }
    }

    /// The state of the agent's session, when it has one.
    #[must_use]
    pub fn state(&self, agent_id: AgentId) -> Option<HookState> {
        self.locked()
            .get(&agent_id)
            .map(|record| record.state.clone())
    }

    /// True while the agent's exact session has a permission ask open.
    #[must_use]
    pub fn ask_open(&self, agent_id: AgentId) -> bool {
        self.state(agent_id).is_some_and(|state| state.ask_open)
    }

    /// True while the office must not type into the agent's terminal because
    /// of what a report said: a permission ask is open or a compaction runs
    /// (0024/FR-016).
    #[must_use]
    pub fn blocks_typing(&self, agent_id: AgentId) -> bool {
        self.state(agent_id)
            .is_some_and(|state| state.ask_open || state.compacting)
    }

    /// Whether a terminal event of the agent reaches the status machine. In
    /// an exact session output and the quiet period do not decide, except
    /// for a turn that is open and silent for a whole quiet period: its
    /// report was lost or the person interrupted it, so the session turns
    /// late and the quiet period decides again (0024/FR-014).
    pub fn judge_terminal_event(&self, agent_id: AgentId, event: TerminalEvent) -> TerminalVerdict {
        let (verdict, turned_late) = self.judge_locked(agent_id, event);
        if let Some(state) = turned_late {
            self.notify(agent_id, Some(state));
        }
        verdict
    }

    /// The verdict, and the new state when the event turned the session late.
    fn judge_locked(
        &self,
        agent_id: AgentId,
        event: TerminalEvent,
    ) -> (TerminalVerdict, Option<HookState>) {
        let mut sessions = self.locked();
        let Some(record) = sessions.get_mut(&agent_id) else {
            return (TerminalVerdict::Pass, None);
        };
        if record.state.source != StatusSource::Exact {
            return (TerminalVerdict::Pass, None);
        }
        match event {
            TerminalEvent::OutputResumed => (TerminalVerdict::Drop, None),
            TerminalEvent::QuietPeriod => {
                if !record.state.works() {
                    return (TerminalVerdict::Drop, None);
                }
                record.state.source = StatusSource::Late;
                record.state.reason = Some(STALLED_TURN_REASON.to_owned());
                // The report that would have closed the work was lost, so the work
                // is presumed over: a later report starts from a clean state.
                record.state.turn_open = false;
                record.state.compacting = false;
                self.waits.set_exact(agent_id, false);
                (TerminalVerdict::Pass, Some(record.state.clone()))
            }
            _ => (TerminalVerdict::Pass, None),
        }
    }

    /// Applies a report of the session that runs in `run`, and says what the
    /// status machine does with it. A report of a helper sub-agent, of a
    /// session that is not running, or of an earlier run changes nothing.
    pub fn apply_report(&self, report: &HookReport, run: RunId) -> Option<ReportOutcome> {
        if report.from_helper {
            return None;
        }
        let (outcome, before, after) = self.apply_locked(report, run)?;
        if before != after {
            self.notify(report.agent_id, Some(after));
        }
        Some(outcome)
    }

    /// The outcome of the report, and the state before and after it.
    fn apply_locked(
        &self,
        report: &HookReport,
        run: RunId,
    ) -> Option<(ReportOutcome, HookState, HookState)> {
        let mut sessions = self.locked();
        let record = sessions.get_mut(&report.agent_id)?;
        if record.run != Some(run) {
            return None;
        }
        let before = record.state.clone();
        if record.state.source != StatusSource::Exact {
            record.state.source = StatusSource::Exact;
            record.state.reason = None;
            self.waits.set_exact(report.agent_id, true);
        }
        let state = &mut record.state;
        // Every report but a permission ask ends an open ask: the CLI went on.
        state.ask_open = report.kind == HookEventKind::PermissionAsked;
        match report.kind {
            HookEventKind::TurnStarted => state.turn_open = true,
            HookEventKind::TurnStopped | HookEventKind::TurnFailed => state.turn_open = false,
            HookEventKind::CompactionStarted => state.compacting = true,
            HookEventKind::CompactionEnded => state.compacting = false,
            HookEventKind::PermissionAsked => {}
        }
        let outcome = ReportOutcome {
            status_event: if state.works() {
                StatusEvent::Output
            } else {
                StatusEvent::QuietPeriodElapsed
            },
            detail: waiting_detail(report.kind, state.works()),
            ask_opened: state.ask_open,
        };
        let after = state.clone();
        Some((outcome, before, after))
    }

    /// A person's input reached the terminal: it ends an open permission
    /// ask, and the status event to apply follows (0024/FR-013).
    pub fn input_ends_ask(&self, agent_id: AgentId) -> Option<StatusEvent> {
        let (event, state) = {
            let mut sessions = self.locked();
            let state = &mut sessions.get_mut(&agent_id)?.state;
            if state.source != StatusSource::Exact || !state.ask_open {
                return None;
            }
            state.ask_open = false;
            let event = if state.works() {
                StatusEvent::Output
            } else {
                StatusEvent::QuietPeriodElapsed
            };
            (event, state.clone())
        };
        self.notify(agent_id, Some(state));
        Some(event)
    }
}

/// The reason that the details show for a late session.
const STALLED_TURN_REASON: &str = "no report for a stalled turn";

/// The word that a change to `waiting for input` carries, for the report
/// that caused it.
fn waiting_detail(kind: HookEventKind, works: bool) -> Option<&'static str> {
    if works {
        return None;
    }
    match kind {
        HookEventKind::PermissionAsked => Some(DETAIL_PERMISSION_ASKED),
        HookEventKind::TurnFailed => Some(DETAIL_TURN_FAILED),
        HookEventKind::TurnStopped | HookEventKind::CompactionEnded => Some(DETAIL_TURN_STOPPED),
        HookEventKind::TurnStarted | HookEventKind::CompactionStarted => None,
    }
}

#[cfg(test)]
#[path = "hook_sessions.tests.rs"]
mod tests;
