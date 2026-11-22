// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use kernel::AgentId;
use terminal_application::common::models::RunId;

/// How long after a person's last input the office keeps directives out of
/// that terminal (0012/FR-002).
pub const PERSON_TYPING_PAUSE: Duration = Duration::from_secs(30);

#[derive(Default)]
struct TerminalInput {
    last_input: Option<Instant>,
    /// The run into which the office typed text without Enter that waits
    /// for the person: a startup prompt or an unsubmitted directive.
    pending_office_text: Option<RunId>,
}

/// What people type into each terminal, as far as directives need it: when
/// they last typed, and whether text the office typed without Enter still
/// waits for them (0012 A11). Clones share one record.
#[derive(Clone, Default)]
pub struct PersonInput {
    terminals: Arc<Mutex<HashMap<AgentId, TerminalInput>>>,
}

impl PersonInput {
    fn locked(&self) -> MutexGuard<'_, HashMap<AgentId, TerminalInput>> {
        self.terminals
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// A person's input reached the terminal: it also ends the wait of any
    /// office text, since the person has seen the terminal.
    pub fn stamp(&self, agent_id: AgentId) {
        self.stamp_at(agent_id, Instant::now());
    }

    fn stamp_at(&self, agent_id: AgentId, at: Instant) {
        let mut terminals = self.locked();
        let terminal = terminals.entry(agent_id).or_default();
        terminal.last_input = Some(at);
        terminal.pending_office_text = None;
    }

    /// The office typed text without Enter into `run`, which waits for the
    /// person until their next input or the run's end.
    pub fn office_text_left(&self, agent_id: AgentId, run: RunId) {
        self.locked()
            .entry(agent_id)
            .or_default()
            .pending_office_text = Some(run);
    }

    /// True while a person typed within the pause, or office text waits for
    /// them in the agent's current run. Text from an older run is ignored.
    #[must_use]
    pub fn holds_directives(&self, agent_id: AgentId, current_run: Option<RunId>) -> bool {
        self.holds_directives_at(agent_id, current_run, Instant::now())
    }

    fn holds_directives_at(
        &self,
        agent_id: AgentId,
        current_run: Option<RunId>,
        now: Instant,
    ) -> bool {
        let terminals = self.locked();
        let Some(terminal) = terminals.get(&agent_id) else {
            return false;
        };
        let typed_recently = terminal
            .last_input
            .is_some_and(|at| now.duration_since(at) < PERSON_TYPING_PAUSE);
        let text_waits =
            terminal.pending_office_text.is_some() && terminal.pending_office_text == current_run;
        typed_recently || text_waits
    }
}

#[cfg(test)]
#[path = "person_input.tests.rs"]
mod tests;
