// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use kernel::AgentId;
use terminal_application::common::models::RunId;
use terminal_domain::services::InputDraft;

/// How long after a person's last input the office keeps directives out of
/// that terminal (0012/FR-002).
pub const PERSON_TYPING_PAUSE: Duration = Duration::from_secs(30);

#[derive(Default)]
struct TerminalInput {
    last_input: Option<Instant>,
    /// What the person typed at the input line and has not sent or cleared
    /// (0012 A5).
    draft: InputDraft,
    /// The run into which the office typed text without Enter that waits
    /// for the person: a startup prompt or an unsubmitted directive.
    pending_office_text: Option<RunId>,
    /// When the office last typed a directive in (0019 A5).
    last_directive: Option<Instant>,
}

/// What people type into each terminal, as far as directives need it: when
/// they last typed, whether text they typed waits unsent (0012 A5), and
/// whether text the office typed without Enter still waits for them (0012
/// A11). Clones share one record.
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

    /// A person's input reached the terminal: it updates their unsent text,
    /// and it ends the wait of any office text, since the person has seen
    /// the terminal.
    pub fn stamp(&self, agent_id: AgentId, bytes: &[u8]) {
        self.stamp_at(agent_id, bytes, Instant::now());
    }

    fn stamp_at(&self, agent_id: AgentId, bytes: &[u8], at: Instant) {
        let mut terminals = self.locked();
        let terminal = terminals.entry(agent_id).or_default();
        terminal.last_input = Some(at);
        terminal.draft.feed(bytes);
        terminal.pending_office_text = None;
    }

    /// True when a person's input reached the agent's terminal at or after
    /// `since` (0019 A5).
    #[must_use]
    pub fn typed_since(&self, agent_id: AgentId, since: Instant) -> bool {
        self.locked()
            .get(&agent_id)
            .and_then(|terminal| terminal.last_input)
            .is_some_and(|at| at >= since)
    }

    /// The office is about to type a directive into the agent's terminal.
    pub fn directive_typed(&self, agent_id: AgentId) {
        self.locked().entry(agent_id).or_default().last_directive = Some(Instant::now());
    }

    /// True when the office typed a directive into the agent's terminal at
    /// or after `since` (0019 A5).
    #[must_use]
    pub fn directive_typed_since(&self, agent_id: AgentId, since: Instant) -> bool {
        self.locked()
            .get(&agent_id)
            .and_then(|terminal| terminal.last_directive)
            .is_some_and(|at| at >= since)
    }

    /// A new session starts with an empty input line.
    pub fn session_started(&self, agent_id: AgentId) {
        self.locked().remove(&agent_id);
    }

    /// The office typed text without Enter into `run`, which waits for the
    /// person until their next input or the run's end.
    pub fn office_text_left(&self, agent_id: AgentId, run: RunId) {
        self.locked()
            .entry(agent_id)
            .or_default()
            .pending_office_text = Some(run);
    }

    /// True while a person typed within the pause, their typed text waits
    /// unsent, or office text waits for them in the agent's current run.
    /// Office text from an older run is ignored.
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
        typed_recently || terminal.has_waiting_text(current_run)
    }

    /// True while the person's typed text waits unsent at the input line, or
    /// office text waits for them in the agent's current run. A sleep would
    /// lose that text (0018/FR-002 e).
    #[must_use]
    pub fn holds_text(&self, agent_id: AgentId, current_run: Option<RunId>) -> bool {
        self.locked()
            .get(&agent_id)
            .is_some_and(|terminal| terminal.has_waiting_text(current_run))
    }

    /// How long ago a person's input last reached the agent's terminal, or
    /// `None` when none did since the server started.
    #[must_use]
    pub fn since_last_input(&self, agent_id: AgentId) -> Option<Duration> {
        self.locked()
            .get(&agent_id)
            .and_then(|terminal| terminal.last_input)
            .map(|at| at.elapsed())
    }
}

impl TerminalInput {
    /// Office text from an older run is ignored.
    fn has_waiting_text(&self, current_run: Option<RunId>) -> bool {
        let office_text_waits =
            self.pending_office_text.is_some() && self.pending_office_text == current_run;
        self.draft.is_unsent() || office_text_waits
    }
}

#[cfg(test)]
#[path = "person_input.tests.rs"]
mod tests;
