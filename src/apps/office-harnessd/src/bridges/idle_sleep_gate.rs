// SPDX-License-Identifier: AGPL-3.0-or-later

//! The gate of idle sleep (0018/FR-002): whether an idle agent may be put to
//! sleep now, and which agents are going to sleep.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use agent_application::common::interfaces::{AgentStore, SleepGate};
use hierarchy_application::{HoldService, SendDirectiveHandler, TreeQuery};
use hierarchy_domain::{Hold, TreeRow, hold_refusal};
use kernel::AgentId;
use notification_presentation::NeedsYouUseCases;
use terminal_application::common::interfaces::TerminalRuntime;

use super::{NeedsYouBridge, PersonInput};

const NOT_CONNECTED: &str = "the office is still starting";
const SLEEP_OFF: &str = "idle sleep is off";
const WAITS_IN_NEEDS_YOU: &str = "it waits in Needs you";
const HOLD_COVERS_IT: &str = "a hold covers it";
const QUEUE_NOT_EMPTY: &str = "a directive or a report waits in its queue";
const TEXT_WAITS: &str = "text waits at its input line";
const CHILD_RUNS: &str = "one of its children runs";

/// The agents that are going to sleep: from the office's last check until
/// their CLI has ended (0018 Terms). Directives and reports to them are
/// refused as asleep (FR-005). Clones share one set.
#[derive(Clone, Default)]
pub struct SleepingAgents {
    agents: Arc<Mutex<HashSet<AgentId>>>,
}

impl SleepingAgents {
    fn locked(&self) -> MutexGuard<'_, HashSet<AgentId>> {
        self.agents.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[must_use]
    pub fn contains(&self, agent_id: AgentId) -> bool {
        self.locked().contains(&agent_id)
    }

    fn insert(&self, agent_id: AgentId) {
        self.locked().insert(agent_id);
    }

    fn remove(&self, agent_id: AgentId) {
        self.locked().remove(&agent_id);
    }
}

/// The idle time in force: how long an agent must be idle, and the moment
/// from which the setting lets the time count (the last time the person
/// turned idle sleep on or changed the idle time).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdleRule {
    pub idle_time: Duration,
    pub counted_from: Instant,
}

/// What the checks read. These services are built after the agent module
/// that holds the gate, so they are connected later.
pub struct GateInputs {
    pub agents: Arc<dyn AgentStore>,
    pub tree: Arc<TreeQuery>,
    pub holds: Arc<HoldService>,
    pub needs_you: NeedsYouBridge,
    pub directives: Arc<SendDirectiveHandler>,
    pub person_input: PersonInput,
    pub runtime: Arc<dyn TerminalRuntime>,
    /// The quiet period the terminals run with: an agent shows `waiting for
    /// input` this long after its last output.
    pub quiet_period: Duration,
}

/// Decides whether an idle agent may be put to sleep. The idle sleep task
/// asks it first, and the sleep step asks it again under the status order
/// right before the stop. It takes no status-order lock itself, so the
/// second ask cannot deadlock.
#[derive(Default)]
pub struct IdleSleepGate {
    sleeping: SleepingAgents,
    rule: Mutex<Option<IdleRule>>,
    inputs: OnceLock<GateInputs>,
}

impl IdleSleepGate {
    /// The going-to-sleep set, for the directive bridge.
    #[must_use]
    pub fn sleeping(&self) -> SleepingAgents {
        self.sleeping.clone()
    }

    /// Connects the services the checks read. Until then, no agent sleeps.
    pub fn connect(&self, inputs: GateInputs) {
        // Startup connects the gate once; a later call has nothing to change.
        let _ = self.inputs.set(inputs);
    }

    /// The idle time in force, or `None` while idle sleep is off.
    pub fn set_rule(&self, rule: Option<IdleRule>) {
        *self.rule.lock().unwrap_or_else(PoisonError::into_inner) = rule;
    }

    fn rule(&self) -> Option<IdleRule> {
        *self.rule.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// How long the agent has shown no activity: no output beyond the quiet
    /// period and no person input. `None` until the gate is connected.
    #[must_use]
    pub fn quiet_for(&self, agent_id: AgentId) -> Option<Duration> {
        self.inputs
            .get()
            .map(|inputs| inputs.without_activity(agent_id))
    }

    /// `Err` with the first blocking condition of FR-002 that holds.
    fn check(&self, agent_id: AgentId) -> Result<(), String> {
        let inputs = self.inputs.get().ok_or(NOT_CONNECTED)?;
        let rule = self.rule().ok_or(SLEEP_OFF)?;
        let idle_for = inputs
            .without_activity(agent_id)
            .min(rule.counted_from.elapsed());
        if idle_for < rule.idle_time {
            return Err(format!(
                "idle for {} s of {} s",
                idle_for.as_secs(),
                rule.idle_time.as_secs()
            ));
        }
        inputs.check_attention(agent_id)?;
        inputs.check_input_line(agent_id)?;
        inputs.check_tree(agent_id)
    }
}

impl SleepGate for IdleSleepGate {
    fn blocking_reason(&self, agent_id: AgentId) -> Option<String> {
        self.check(agent_id).err()
    }

    fn begin(&self, agent_id: AgentId) {
        self.sleeping.insert(agent_id);
    }

    fn end(&self, agent_id: AgentId) {
        self.sleeping.remove(agent_id);
    }
}

impl GateInputs {
    /// The shorter of the time since the agent changed to `waiting for
    /// input` and the time since the last person input (Terms: Idle since).
    fn without_activity(&self, agent_id: AgentId) -> Duration {
        let since_waiting = self
            .runtime
            .quiet_for(agent_id)
            .unwrap_or_default()
            .saturating_sub(self.quiet_period);
        let since_input = self
            .person_input
            .since_last_input(agent_id)
            .unwrap_or(Duration::MAX);
        since_waiting.min(since_input)
    }

    /// FR-002 b: an item in Needs you.
    fn check_attention(&self, agent_id: AgentId) -> Result<(), String> {
        let items = self.needs_you.list().map_err(|error| error.to_string())?;
        if items.iter().any(|item| item.agent_id == agent_id) {
            return Err(WAITS_IN_NEEDS_YOU.to_owned());
        }
        Ok(())
    }

    /// FR-002 d and e: a waiting directive or report, or text that a stop
    /// would lose.
    fn check_input_line(&self, agent_id: AgentId) -> Result<(), String> {
        if self.directives.waiting_count(agent_id) > 0 {
            return Err(QUEUE_NOT_EMPTY.to_owned());
        }
        let current_run = self.runtime.current_run(agent_id);
        if self.person_input.holds_text(agent_id, current_run) {
            return Err(TEXT_WAITS.to_owned());
        }
        Ok(())
    }

    /// FR-002 c and f: a hold that covers the agent, or a running child.
    fn check_tree(&self, agent_id: AgentId) -> Result<(), String> {
        let rows = self
            .tree
            .store_snapshot()
            .map_err(|error| error.to_string())?;
        let view = self.holds.list();
        let holds: Vec<Hold> = view.office.into_iter().chain(view.agents).collect();
        if hold_refusal(&holds, &rows, agent_id, None).is_some() {
            return Err(HOLD_COVERS_IT.to_owned());
        }
        if self.has_running_child(&rows, agent_id)? {
            return Err(CHILD_RUNS.to_owned());
        }
        Ok(())
    }

    fn has_running_child(&self, rows: &[TreeRow], agent_id: AgentId) -> Result<bool, String> {
        let children: HashSet<AgentId> = rows
            .iter()
            .filter(|row| row.parent_agent_id == Some(agent_id))
            .map(|row| row.agent_id)
            .collect();
        if children.is_empty() {
            return Ok(false);
        }
        let agents = self.agents.list().map_err(|error| error.to_string())?;
        Ok(agents
            .iter()
            .any(|agent| children.contains(&agent.id) && agent.status.is_running()))
    }
}

#[cfg(test)]
#[path = "idle_sleep_gate.tests.rs"]
mod tests;
