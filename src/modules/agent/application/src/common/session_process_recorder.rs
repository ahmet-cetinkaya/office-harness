// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use agent_domain::entities::Session;
use agent_domain::value_objects::SessionId;
use kernel::Actor;

use crate::common::action_journal::ActionJournal;
use crate::common::actor_scope::ActorScope;
use crate::common::errors::PortError;
use crate::common::interfaces::{FailureReporter, ProcessProbe, SessionProcessStore, SessionStore};
use crate::common::models::{AgentAction, ProbeError, ProcessKind, SessionProcess, StoredProcess};

/// Open helper rows one session may hold (0017/FR-010).
pub const MAX_OPEN_HELPERS_PER_SESSION: usize = 200;

/// A command line can read as empty for a moment right after a spawn on
/// Linux, so the first read of the CLI is repeated before it counts as lost.
const FIRST_READ_ATTEMPTS: u32 = 10;
const FIRST_READ_PAUSE: Duration = Duration::from_millis(50);

/// Stores the identity of the CLI process of a session at its start, and
/// keeps the identities of its helpers current while it runs (0017 D4).
#[derive(Clone)]
pub struct SessionProcessRecorder {
    sessions: Arc<dyn SessionStore>,
    store: Arc<dyn SessionProcessStore>,
    probe: Arc<dyn ProcessProbe>,
    journal: ActionJournal,
    failures: Arc<dyn FailureReporter>,
    /// Sessions whose helper limit entry is written, so it is written once.
    limited: Arc<Mutex<HashSet<SessionId>>>,
}

/// What one scan knows about the rows of a session before it looks for new
/// helpers.
struct KnownProcesses {
    pids: HashSet<u32>,
    open_helpers: usize,
}

impl SessionProcessRecorder {
    #[must_use]
    pub fn new(
        sessions: Arc<dyn SessionStore>,
        store: Arc<dyn SessionProcessStore>,
        probe: Arc<dyn ProcessProbe>,
        journal: ActionJournal,
        failures: Arc<dyn FailureReporter>,
    ) -> Self {
        Self {
            sessions,
            store,
            probe,
            journal,
            failures,
            limited: Arc::default(),
        }
    }

    /// Stores the CLI identity of a session that just started. A failure
    /// stores nothing and is reported; the session keeps running and later
    /// counts as having no complete stored identities (0017/FR-001).
    pub fn record_cli(&self, session: &Session) {
        if let Err(error) = self.store_cli(session) {
            self.failures.report(&error);
        }
    }

    /// One pass over every running session: new helpers are stored, a helper
    /// that ended or became another process is dropped, and a row whose
    /// process replaced itself with another program takes the new command.
    pub fn scan(&self) {
        let running = match self.sessions.running() {
            Ok(running) => running,
            Err(error) => return self.failures.report(&error),
        };
        self.forget_ended(&running);
        for session in &running {
            if let Err(error) = self.scan_session(session) {
                self.failures.report(&error);
            }
        }
    }

    fn store_cli(&self, session: &Session) -> Result<(), PortError> {
        let process = self
            .probe
            .cli_process_id(session.agent_id)
            .and_then(|pid| self.read_after_spawn(pid))
            .ok_or_else(|| PortError::NotFound {
                what: format!("the CLI process of session {}", session.id),
            })?;
        self.store
            .insert_open(session.id, ProcessKind::Cli, &process)?;
        Ok(())
    }

    fn read_after_spawn(&self, pid: u32) -> Option<SessionProcess> {
        for attempt in 1..=FIRST_READ_ATTEMPTS {
            match self.probe.read(pid) {
                Ok(process) => return Some(process),
                Err(ProbeError::NotRunning) => return None,
                Err(ProbeError::NotReadable) if attempt < FIRST_READ_ATTEMPTS => {
                    thread::sleep(FIRST_READ_PAUSE);
                }
                Err(ProbeError::NotReadable) => {}
            }
        }
        None
    }

    fn scan_session(&self, session: &Session) -> Result<(), PortError> {
        let stored = self.store.open_for_session(session.id)?;
        let mut known = self.refresh_stored(&stored)?;
        let Some(root) = self.probe.cli_process_id(session.agent_id) else {
            return Ok(());
        };
        for pid in self.probe.descendants(root) {
            if known.pids.contains(&pid) {
                continue;
            }
            let Ok(process) = self.probe.read(pid) else {
                continue;
            };
            if known.open_helpers >= MAX_OPEN_HELPERS_PER_SESSION {
                return self.note_helper_limit(session);
            }
            if self
                .store
                .insert_open(session.id, ProcessKind::Helper, &process)?
            {
                known.pids.insert(pid);
                known.open_helpers += 1;
            }
        }
        Ok(())
    }

    /// Brings the stored rows in line with the live processes and reports
    /// which process ids stay stored.
    fn refresh_stored(&self, stored: &[StoredProcess]) -> Result<KnownProcesses, PortError> {
        let mut known = KnownProcesses {
            pids: HashSet::new(),
            open_helpers: 0,
        };
        for row in stored {
            if self.keeps_row(row)? {
                known.pids.insert(row.process.pid);
                known.open_helpers += usize::from(row.kind == ProcessKind::Helper);
            }
        }
        Ok(known)
    }

    /// False when the row was dropped. A helper that left the tree stays
    /// while it matches (0017/FR-002); a CLI row is left for the session end.
    fn keeps_row(&self, row: &StoredProcess) -> Result<bool, PortError> {
        match self.probe.read(row.process.pid) {
            Ok(live) if live.started_at == row.process.started_at => {
                if live.command_hash != row.process.command_hash {
                    self.store.update_command(row.id, &live)?;
                }
                Ok(true)
            }
            Ok(_) | Err(ProbeError::NotRunning) if row.kind == ProcessKind::Helper => {
                self.store.drop_helper(row.id)?;
                Ok(false)
            }
            Ok(_) | Err(_) => Ok(true),
        }
    }

    /// Writes the one action record entry of a session that reached the
    /// helper limit, and marks the session so that a sweep cannot treat its
    /// stored identities as complete.
    fn note_helper_limit(&self, session: &Session) -> Result<(), PortError> {
        if !self.limited().insert(session.id) {
            return Ok(());
        }
        self.store.mark_helper_limit(session.id)?;
        let detail = format!(
            "the office stores at most {MAX_OPEN_HELPERS_PER_SESSION} helper processes for a \
             session and stopped storing more"
        );
        let _office = ActorScope::enter(Actor::Office);
        self.journal
            .record_detailed(
                session.agent_id,
                AgentAction::HelperLimit,
                None,
                Some(&detail),
            )
            .map_err(|error| PortError::Storage {
                message: error.to_string(),
                is_disk_failure: false,
            })
    }

    fn forget_ended(&self, running: &[Session]) {
        self.limited()
            .retain(|id| running.iter().any(|session| session.id == *id));
    }

    fn limited(&self) -> std::sync::MutexGuard<'_, HashSet<SessionId>> {
        self.limited.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
#[path = "session_process_recorder.tests.rs"]
mod tests;
