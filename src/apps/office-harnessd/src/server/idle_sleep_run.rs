// SPDX-License-Identifier: AGPL-3.0-or-later

//! The idle sleep task (0018): a few times a minute it puts the agents that
//! have been idle for the idle time, and that no blocking condition keeps
//! awake, to sleep.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use agent_application::common::actor_scope::ActorScope;
use agent_application::common::interfaces::{AgentStore, SettingsStore, SleepGate};
use agent_domain::enums::AgentStatus;
use kernel::{Actor, AgentId};
use serde_json::{Map, json};
use tokio::sync::watch;
use tokio::task::spawn_blocking;
use tokio::time::{MissedTickBehavior, interval};
use wire::{ErrorCode, Request, Response};

use super::agent_requests::AgentGateway;
use crate::bootstrap::ServerLog;
use crate::bridges::{IdleRule, IdleSleepGate};

const SLEEP_REQUEST: &str = "agent.sleep";
/// Requests that the server sends itself never reach a client, so their id
/// is never matched with an answer.
const OWN_REQUEST_ID: u32 = 0;
const SETTING_OFF: u32 = 0;

/// What the task remembers between two rounds.
struct RoundState {
    /// The idle time the last round saw, in minutes; `None` before the
    /// first round.
    seen_minutes: Option<u32>,
    /// The last time the setting changed, or the server start (Terms: Idle
    /// since).
    changed_at: Instant,
    /// Agents whose sleep failed, with the moment (FR-009).
    failed: HashMap<AgentId, Instant>,
}

impl RoundState {
    fn new(started_at: Instant) -> Self {
        Self {
            seen_minutes: None,
            changed_at: started_at,
            failed: HashMap::new(),
        }
    }

    /// The idle time in force after this round's look at the setting, or
    /// `None` while it is off.
    fn observe(&mut self, minutes: u32, minute: Duration) -> Option<IdleRule> {
        if self.seen_minutes.is_some_and(|seen| seen != minutes) {
            self.changed_at = Instant::now();
        }
        self.seen_minutes = Some(minutes);
        (minutes != SETTING_OFF).then(|| IdleRule {
            idle_time: minute * minutes,
            counted_from: self.changed_at,
        })
    }

    /// True while a failed sleep still bars the agent: it has had no
    /// prompted wait since the failure (FR-009). Once it has, the normal
    /// check counts the idle time from that wait.
    fn bars(&mut self, agent_id: AgentId, last_prompted_wait: Option<Instant>) -> bool {
        let Some(failed_at) = self.failed.get(&agent_id) else {
            return false;
        };
        let has_waited_since = last_prompted_wait.is_some_and(|waited_at| waited_at > *failed_at);
        if has_waited_since {
            self.failed.remove(&agent_id);
        }
        !has_waited_since
    }
}

/// The idle sleep task. Clones share one round state.
#[derive(Clone)]
pub struct IdleSleeper {
    gate: Arc<IdleSleepGate>,
    settings: Arc<dyn SettingsStore>,
    store: Arc<dyn AgentStore>,
    agents: AgentGateway,
    tick: Duration,
    /// The length of one minute of the setting; tests shorten it.
    minute: Duration,
    log: ServerLog,
    state: Arc<Mutex<RoundState>>,
}

impl IdleSleeper {
    #[must_use]
    pub fn new(
        gate: Arc<IdleSleepGate>,
        settings: Arc<dyn SettingsStore>,
        store: Arc<dyn AgentStore>,
        agents: AgentGateway,
        tick: Duration,
        minute: Duration,
        log: ServerLog,
    ) -> Self {
        Self {
            gate,
            settings,
            store,
            agents,
            tick,
            minute,
            log,
            state: Arc::new(Mutex::new(RoundState::new(Instant::now()))),
        }
    }

    /// Runs until the server stops: `stopping` turns true or its sender is
    /// gone. A server started again in the same process runs its own task.
    pub fn spawn(self, mut stopping: watch::Receiver<bool>) {
        tokio::spawn(async move {
            let mut ticks = interval(self.tick);
            ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = ticks.tick() => {}
                    _ = stopping.wait_for(|stopped| *stopped) => return,
                }
                let sleeper = self.clone();
                // Every read and the stop block, so a round leaves the async
                // threads.
                if let Err(error) = spawn_blocking(move || sleeper.round()).await {
                    self.log
                        .error(&format!("an idle sleep round failed: {error}"));
                }
            }
        });
    }

    /// One look at every agent. The sleeps run one after another; each
    /// frees the status order before the next, so a person's request waits
    /// for at most one stop grace time.
    fn round(&self) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let minutes = match self.settings.load() {
            Ok(settings) => settings.idle_sleep_minutes,
            Err(error) => {
                return self
                    .log
                    .error(&format!("idle sleep cannot read the settings: {error}"));
            }
        };
        let rule = state.observe(minutes, self.minute);
        self.gate.set_rule(rule);
        if rule.is_none() {
            state.failed.clear();
            return;
        }
        let agents = match self.store.list() {
            Ok(agents) => agents,
            Err(error) => {
                return self
                    .log
                    .error(&format!("idle sleep cannot read the agents: {error}"));
            }
        };
        let idle = agents
            .iter()
            .filter(|agent| agent.status == AgentStatus::WaitingForInput);
        for agent in idle {
            if state.bars(agent.id, self.gate.last_prompted_wait(agent.id))
                || self.gate.blocking_reason(agent.id).is_some()
            {
                continue;
            }
            if let Response::Failure {
                code: ErrorCode::Internal,
                ..
            } = self.sleep(agent.id, minutes)
            {
                state.failed.insert(agent.id, Instant::now());
            }
        }
    }

    /// Runs `agent.sleep` as the office through the agent gateway, which
    /// keeps the status order, the events, and the action record.
    fn sleep(&self, agent_id: AgentId, minutes: u32) -> Response {
        let mut fields = Map::new();
        fields.insert("agent_id".to_owned(), json!(agent_id.to_string()));
        fields.insert("idle_minutes".to_owned(), json!(minutes));
        let request = Request {
            id: OWN_REQUEST_ID,
            kind: SLEEP_REQUEST.to_owned(),
            fields,
        };
        let _scope = ActorScope::enter(Actor::Office);
        self.agents.run(&request)
    }
}

#[cfg(test)]
#[path = "idle_sleep_run.tests.rs"]
mod tests;
