// SPDX-License-Identifier: AGPL-3.0-or-later

//! The watch of a new session's first seconds for the CLI's sign-in screen
//! (0019/FR-005, D9). When the screen shows, the watch marks the run as not
//! signed in and stops the child; the status bridge then ends the session
//! as not signed in when the exit arrives, under the status order.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use agent_application::common::models::SignIn;
use agent_application::common::sign_in_check::{
    NotSignedInSource, line_with_word, not_signed_in_reason, quoted_line,
};
use kernel::AgentId;
use terminal_application::common::errors::TerminalRuntimeError;
use terminal_application::common::interfaces::TerminalRuntime;
use terminal_application::common::models::RunId;
use terminal_domain::value_objects::ScreenView;

use super::person_input::PersonInput;
use crate::bootstrap::ServerLog;

/// How long a new session is watched (0019 A5).
pub const SIGN_IN_WATCH_PERIOD: Duration = Duration::from_secs(20);
/// How often the screen is read while the watch runs.
pub const SIGN_IN_WATCH_POLL: Duration = Duration::from_millis(250);
/// How long the CLI may take to end after the stop request. A sign-in
/// screen holds no work to save.
const SIGN_IN_STOP_GRACE: Duration = Duration::from_secs(2);
/// The bottom rows that show anything, as the question check reads them
/// (0012, 0019 A5).
const SIGN_IN_ROWS: usize = 15;

/// How long and how often the watch reads the screen. Tests shorten it.
#[derive(Debug, Clone, Copy)]
pub struct SignInWatchTiming {
    pub period: Duration,
    pub poll: Duration,
}

impl Default for SignInWatchTiming {
    fn default() -> Self {
        Self {
            period: SIGN_IN_WATCH_PERIOD,
            poll: SIGN_IN_WATCH_POLL,
        }
    }
}

/// The runs that the watch stopped at a sign-in screen, with the reason,
/// until the status bridge takes them at the run's exit. Clones share one
/// record.
#[derive(Clone, Default)]
pub struct SignInEnds {
    runs: Arc<Mutex<HashMap<AgentId, (RunId, String)>>>,
}

impl SignInEnds {
    fn locked(&self) -> MutexGuard<'_, HashMap<AgentId, (RunId, String)>> {
        self.runs.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn mark(&self, agent_id: AgentId, run: RunId, reason: String) {
        self.locked().insert(agent_id, (run, reason));
    }

    /// Drops the mark of a run that ended without the status bridge taking
    /// it, so a later exit of the same run is never read as not signed in.
    pub fn forget(&self, agent_id: AgentId, run: RunId) {
        let _ = self.take(agent_id, run);
    }

    /// The reason when the watch stopped exactly this run, removed from the
    /// record.
    #[must_use]
    pub fn take(&self, agent_id: AgentId, run: RunId) -> Option<String> {
        let mut runs = self.locked();
        if runs
            .get(&agent_id)
            .is_some_and(|(marked, _)| *marked == run)
        {
            return runs.remove(&agent_id).map(|(_, reason)| reason);
        }
        None
    }
}

/// What one watch needs from the server.
#[derive(Clone)]
pub struct SignInWatch {
    pub runtime: Arc<dyn TerminalRuntime>,
    pub person_input: PersonInput,
    pub ends: SignInEnds,
    pub timing: SignInWatchTiming,
    pub log: ServerLog,
}

impl SignInWatch {
    /// Watches the agent's current run in the background. The words are the
    /// CLI's own; a CLI without words, or a session whose startup prompt
    /// holds one of them, is never watched (0019/FR-011).
    pub fn start(&self, agent_id: AgentId, sign_in: SignIn, startup_prompt: Option<&str>) {
        let prompt_holds_a_word = startup_prompt
            .is_some_and(|prompt| line_with_word(prompt, sign_in.screen_words).is_some());
        if sign_in.screen_words.is_empty() || prompt_holds_a_word {
            return;
        }
        let Some(run) = self.runtime.current_run(agent_id) else {
            return;
        };
        let watch = self.clone();
        let started = Instant::now();
        thread::spawn(move || watch.run(agent_id, run, sign_in, started));
    }

    fn run(&self, agent_id: AgentId, run: RunId, sign_in: SignIn, started: Instant) {
        let deadline = started + self.timing.period;
        while Instant::now() < deadline {
            let is_same_run = self.runtime.current_run(agent_id) == Some(run);
            let someone_typed = self.person_input.typed_since(agent_id, started)
                || self.person_input.directive_typed_since(agent_id, started);
            if !is_same_run || someone_typed {
                return;
            }
            let row = self
                .runtime
                .screen(agent_id)
                .and_then(|screen| sign_in_row(&screen, sign_in.screen_words));
            if let Some(row) = row {
                let reason = not_signed_in_reason(&sign_in, NotSignedInSource::SignInScreen, &row);
                self.end(agent_id, run, reason);
                return;
            }
            thread::sleep(self.timing.poll);
        }
    }

    /// Marks the run, then stops it, so the exit always finds the mark. A
    /// run that already ended, for example by a person's stop, keeps no
    /// mark: its exit is not the office's doing.
    fn end(&self, agent_id: AgentId, run: RunId, reason: String) {
        self.ends.mark(agent_id, run, reason);
        match self.runtime.stop(agent_id, SIGN_IN_STOP_GRACE) {
            Ok(()) => {}
            Err(TerminalRuntimeError::NotRunning(_)) => self.ends.forget(agent_id, run),
            Err(error) => {
                self.ends.forget(agent_id, run);
                self.log.error(&format!(
                    "a CLI at its sign-in screen could not be stopped: {error}"
                ));
            }
        }
    }
}

/// The first of the bottom rows that holds one of the words, as a quoted
/// line.
fn sign_in_row(screen: &ScreenView, words: &[&str]) -> Option<String> {
    let rows = screen.bottom_rows(SIGN_IN_ROWS).join("\n");
    line_with_word(&rows, words).map(quoted_line)
}

#[cfg(test)]
#[path = "sign_in_watch.tests.rs"]
mod tests;
