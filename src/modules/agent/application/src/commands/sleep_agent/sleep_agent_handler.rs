// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_domain::entities::Agent;
use agent_domain::enums::AgentStatus;
use kernel::AgentId;

use crate::common::action_journal::ActionJournal;
use crate::common::errors::ApplicationError;
use crate::common::interfaces::{AgentStore, SleepGate};
use crate::common::lookup::require_agent;
use crate::common::models::AgentAction;
use crate::common::session_ender::SessionEnder;

/// Puts an idle agent to sleep (`agent.sleep`, spec 0018): the gate checks
/// again, the CLI ends as it does for a stop, and the status becomes `asleep`.
/// The host runs it under the status order, so no status change comes between
/// the check and the stop.
#[derive(Clone)]
pub struct SleepAgentHandler {
    agents: Arc<dyn AgentStore>,
    ender: SessionEnder,
    journal: ActionJournal,
    gate: Arc<dyn SleepGate>,
}

/// Tells the gate that the sleep step is over on every path out of it,
/// including an early return and a panic.
struct GateScope<'a> {
    gate: &'a dyn SleepGate,
    agent_id: AgentId,
}

impl<'a> GateScope<'a> {
    fn begin(gate: &'a dyn SleepGate, agent_id: AgentId) -> Self {
        gate.begin(agent_id);
        Self { gate, agent_id }
    }
}

impl Drop for GateScope<'_> {
    fn drop(&mut self) {
        self.gate.end(self.agent_id);
    }
}

impl SleepAgentHandler {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        ender: SessionEnder,
        journal: ActionJournal,
        gate: Arc<dyn SleepGate>,
    ) -> Self {
        Self {
            agents,
            ender,
            journal,
            gate,
        }
    }

    /// `idle_minutes` is only the reason in the record.
    ///
    /// # Errors
    /// `NotFound`; `Conflict` when the agent does not wait for input or the
    /// gate names a reason to stay awake, both without a record; `Internal`
    /// when the child cannot be ended, which leaves the status as it was and
    /// records the failure.
    pub fn handle(&self, agent_id: AgentId, idle_minutes: u32) -> Result<Agent, ApplicationError> {
        let mut agent = require_agent(&*self.agents, agent_id)?;
        if agent.status != AgentStatus::WaitingForInput {
            return Err(ApplicationError::Conflict(format!(
                "agent {} is not waiting for input, it is {}",
                agent.name, agent.status
            )));
        }
        if let Some(reason) = self.gate.blocking_reason(agent_id) {
            return Err(ApplicationError::Conflict(reason));
        }
        let _scope = GateScope::begin(&*self.gate, agent_id);
        let outcome = self.ender.sleep(&mut agent);
        let failure = outcome.as_ref().err().map(ToString::to_string);
        let detail = format!("idle for {idle_minutes} minutes");
        self.journal.record_detailed(
            agent_id,
            AgentAction::Sleep,
            failure.as_deref(),
            Some(&detail),
        )?;
        outcome?;
        // The stored status is the truth: a child that ended on its own just
        // before changed nothing here, so the copy read above may be stale.
        require_agent(&*self.agents, agent_id)
    }
}

#[cfg(test)]
#[path = "sleep_agent_handler.tests.rs"]
mod tests;
