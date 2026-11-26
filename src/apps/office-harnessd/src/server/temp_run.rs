// SPDX-License-Identifier: AGPL-3.0-or-later

//! The temp watcher (0020 D8): every tick it looks at the temps and ends
//! the ones whose CLI stopped, exited, or failed, and the ones that wrote
//! nothing for the idle limit. The ender does the end.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use agent_application::common::interfaces::{AgentStore, SessionStore, SettingsStore};
use agent_domain::entities::Agent;
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::ExitResult;
use audit_application::queries::list_actions::ListActionsHandler;
use audit_domain::{ActionKind, ActionResult};
use kernel::{Actor, AgentId};
use tokio::sync::watch;
use tokio::task::spawn_blocking;
use tokio::time::{MissedTickBehavior, interval};

use crate::bootstrap::ServerLog;
use crate::bridges::{EndOrder, EndReason, IdleSleepGate, StatusOrder, TempEnder, TempEndings};

/// The moments since which each temp has been quiet, as the last round saw
/// them. `temp.list` reads the quiet time from here. Clones share one board.
#[derive(Clone, Default)]
pub struct QuietBoard {
    since: Arc<Mutex<HashMap<AgentId, Instant>>>,
}

impl QuietBoard {
    /// How long the temp has been quiet, or `None` when the last round saw it
    /// neither running nor asleep.
    #[must_use]
    pub fn quiet_for(&self, agent_id: AgentId) -> Option<Duration> {
        self.since
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&agent_id)
            .map(Instant::elapsed)
    }

    fn replace(&self, since: HashMap<AgentId, Instant>) {
        *self.since.lock().unwrap_or_else(PoisonError::into_inner) = since;
    }
}

/// What the watcher keeps between two rounds.
#[derive(Default)]
struct RoundState {
    /// The last moment a hold, a running child, or a Needs you item kept the
    /// temp from the idle end (0020/FR-004).
    blocked_at: HashMap<AgentId, Instant>,
    /// The quiet-since moment of the last round, which an asleep temp keeps
    /// because no CLI runs to measure it.
    quiet_since: HashMap<AgentId, Instant>,
}

/// The temp watcher. Clones share one round state.
#[derive(Clone)]
pub struct TempWatcher {
    agents: Arc<dyn AgentStore>,
    sessions: Arc<dyn SessionStore>,
    settings: Arc<dyn SettingsStore>,
    actions: ListActionsHandler,
    status_order: StatusOrder,
    gate: Arc<IdleSleepGate>,
    endings: TempEndings,
    ender: TempEnder,
    board: QuietBoard,
    tick: Duration,
    /// The length of one minute of the idle limit; tests shorten it.
    minute: Duration,
    log: ServerLog,
    state: Arc<Mutex<RoundState>>,
}

/// What a watcher is built from.
pub struct WatcherParts {
    pub agents: Arc<dyn AgentStore>,
    pub sessions: Arc<dyn SessionStore>,
    pub settings: Arc<dyn SettingsStore>,
    pub actions: ListActionsHandler,
    pub status_order: StatusOrder,
    pub gate: Arc<IdleSleepGate>,
    pub endings: TempEndings,
    pub ender: TempEnder,
    pub board: QuietBoard,
    pub tick: Duration,
    pub minute: Duration,
    pub log: ServerLog,
}

impl TempWatcher {
    #[must_use]
    pub fn new(parts: WatcherParts) -> Self {
        Self {
            agents: parts.agents,
            sessions: parts.sessions,
            settings: parts.settings,
            actions: parts.actions,
            status_order: parts.status_order,
            gate: parts.gate,
            endings: parts.endings,
            ender: parts.ender,
            board: parts.board,
            tick: parts.tick,
            minute: parts.minute,
            log: parts.log,
            state: Arc::default(),
        }
    }

    /// Runs until the server stops: `stopping` turns true or its sender is
    /// gone.
    pub fn spawn(self, mut stopping: watch::Receiver<bool>) {
        tokio::spawn(async move {
            let mut ticks = interval(self.tick);
            ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = ticks.tick() => {}
                    _ = stopping.wait_for(|stopped| *stopped) => return,
                }
                let watcher = self.clone();
                // Every read and each end block, so a round leaves the async
                // threads.
                if let Err(error) = spawn_blocking(move || watcher.round()).await {
                    self.log.error(&format!("a temp round failed: {error}"));
                }
            }
        });
    }

    /// One look at every temp. The ends run one after another.
    fn round(&self) {
        let minutes = match self.settings.load() {
            Ok(settings) => settings.temp_idle_minutes,
            Err(error) => {
                return self.log.error(&format!(
                    "the temp watcher cannot read the settings: {error}"
                ));
            }
        };
        let agents = match self.agents.list() {
            Ok(agents) => agents,
            Err(error) => {
                return self
                    .log
                    .error(&format!("the temp watcher cannot read the agents: {error}"));
            }
        };
        // Read after the list: a temp in the list was created before this
        // read, so a create that has not finished its start still shows.
        if self.endings.is_creating() {
            return;
        }
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let mut quiet_since = HashMap::new();
        for agent in agents.iter().filter(|agent| agent.temp.is_some()) {
            if self.ender.is_ending(agent.id) {
                continue;
            }
            match self.verdict(&mut state, agent, minutes) {
                Verdict::Keep => {}
                Verdict::Quiet(since) => {
                    quiet_since.insert(agent.id, since);
                }
                Verdict::End(order) => drop(self.ender.end(&order)),
            }
        }
        state
            .blocked_at
            .retain(|agent_id, _| quiet_since.contains_key(agent_id));
        state.quiet_since.clone_from(&quiet_since);
        self.board.replace(quiet_since);
    }

    fn verdict(&self, state: &mut RoundState, agent: &Agent, minutes: u32) -> Verdict {
        match agent.status {
            AgentStatus::Stopped => end_of(agent, self.stopped_reason(agent.id)),
            AgentStatus::Exited | AgentStatus::Failed => end_of(agent, self.exit_reason(agent)),
            // A restart is no end (0001/FR-004, PB-6).
            AgentStatus::Interrupted => Verdict::Keep,
            AgentStatus::Starting
            | AgentStatus::Working
            | AgentStatus::WaitingForInput
            | AgentStatus::Asleep => self.judge_quiet(state, agent, minutes),
        }
    }

    /// The quiet-since moment, or the end when it is as old as the idle limit.
    fn judge_quiet(&self, state: &mut RoundState, agent: &Agent, minutes: u32) -> Verdict {
        let agent_id = agent.id;
        let now = Instant::now();
        if self.gate.keeps_from_idle_end(agent_id) {
            state.blocked_at.insert(agent_id, now);
        }
        let since = quiet_since(
            now,
            self.gate.output_quiet_for(agent_id),
            state.quiet_since.get(&agent_id).copied(),
            state.blocked_at.get(&agent_id).copied(),
        );
        if idle_limit_reached(now, since, self.minute * minutes) {
            return Verdict::End(idle_end_of(agent, minutes));
        }
        Verdict::Quiet(since)
    }

    fn stopped_reason(&self, agent_id: AgentId) -> EndReason {
        let actor = stop_actor(&self.actions, &self.status_order, agent_id);
        EndReason::Stopped {
            by: self.actor_phrase(actor),
        }
    }

    fn actor_phrase(&self, actor: Actor) -> String {
        match actor {
            Actor::User => "the person".to_owned(),
            Actor::Office => "the office".to_owned(),
            Actor::Agent(agent_id) => self
                .agents
                .find(agent_id)
                .ok()
                .flatten()
                .map_or_else(|| agent_id.to_string(), |agent| agent.name.to_string()),
            Actor::Schedule(schedule_id) => format!("schedule {schedule_id}"),
        }
    }

    /// An exit code is an exit; anything else the CLI left is a failure.
    fn exit_reason(&self, agent: &Agent) -> EndReason {
        let result = self
            .sessions
            .latest_for_agent(agent.id)
            .ok()
            .flatten()
            .and_then(|session| session.exit_result);
        exit_reason_of(result.as_ref())
    }
}

/// Who stopped the temp, from the last `stop` entry of its record, or the
/// office when the record names none. A stop saves the `stopped` status before
/// it writes its entry, and holds the status order until the entry is written,
/// so the wait for the order is what makes the entry visible.
fn stop_actor(
    actions: &ListActionsHandler,
    status_order: &StatusOrder,
    agent_id: AgentId,
) -> Actor {
    drop(status_order.lock().unwrap_or_else(PoisonError::into_inner));
    actions
        .handle(agent_id)
        .ok()
        .and_then(|entries| {
            entries.into_iter().rev().find(|entry| {
                entry.record.action == ActionKind::Stop && entry.record.result == ActionResult::Ok
            })
        })
        .map_or(Actor::Office, |entry| entry.record.actor)
}

/// The end of a temp whose CLI stopped, exited, or failed: it goes on only
/// while the temp has the status the round read (D7 step 0).
fn end_of(agent: &Agent, reason: EndReason) -> Verdict {
    Verdict::End(EndOrder {
        agent_id: agent.id,
        reason,
        actor: Actor::Office,
        seen: Some(agent.status),
    })
}

/// The idle end of a temp. It goes on only while the temp has the status the
/// round read, so a new task that holds the status order first wins and the
/// ender never stops the fresh session (D7 step 0).
fn idle_end_of(agent: &Agent, minutes: u32) -> EndOrder {
    EndOrder {
        agent_id: agent.id,
        reason: EndReason::NoOutput { minutes },
        actor: Actor::Office,
        seen: Some(agent.status),
    }
}

/// The moment a temp has been quiet since (0020 D8): the latest of the last
/// output or input of its running CLI and the last moment something kept it
/// from the idle end. A temp whose CLI does not run (asleep) keeps the moment
/// of the last round; one never seen before starts now.
fn quiet_since(
    now: Instant,
    output_quiet: Option<Duration>,
    last_known: Option<Instant>,
    blocked_at: Option<Instant>,
) -> Instant {
    let measured = output_quiet
        .map(|quiet| now.checked_sub(quiet).unwrap_or(now))
        .or(last_known)
        .unwrap_or(now);
    blocked_at.map_or(measured, |blocked| measured.max(blocked))
}

/// True once the quiet time is as long as the idle limit.
fn idle_limit_reached(now: Instant, since: Instant, limit: Duration) -> bool {
    now.duration_since(since) >= limit
}

/// The reason a finished CLI gives, from the exit result of its last session.
fn exit_reason_of(result: Option<&ExitResult>) -> EndReason {
    match result {
        Some(ExitResult::Code(code)) => EndReason::Exited { code: *code },
        Some(ExitResult::SpawnError(reason) | ExitResult::NotSignedIn(reason)) => {
            EndReason::Failed {
                reason: reason.clone(),
            }
        }
        Some(other) => EndReason::Failed {
            reason: other.to_string(),
        },
        None => EndReason::Failed {
            reason: NO_EXIT_RESULT.to_owned(),
        },
    }
}

const NO_EXIT_RESULT: &str = "the CLI ended without an exit result";

/// What one round decides for one temp.
enum Verdict {
    Keep,
    /// The temp lives; it has been quiet since this moment.
    Quiet(Instant),
    End(EndOrder),
}

#[cfg(test)]
#[path = "temp_run.tests.rs"]
mod tests;
