// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use kernel::AgentId;
use terminal_application::commands::spawn_terminal::validate_spawn_terminal;
use terminal_application::common::errors::TerminalRuntimeError;
use terminal_application::common::interfaces::{
    OutputSubscriber, TerminalEventSink, TerminalRuntime,
};
use terminal_application::common::models::{
    Attachment, RunId, SpawnTerminalRequest, SubscriptionId,
};
use terminal_domain::value_objects::{OutputKey, OutputLimits, TerminalSize};

use crate::services::output_store::OutputStore;
use crate::services::pty_host::spawn_in_pty;
use crate::services::screen_emulator::ScreenEmulator;
use crate::services::terminal_session::{SessionParts, TerminalSession};

/// The settings of the runtime that come from the data folder and the
/// settings table.
#[derive(Debug, Clone)]
pub struct RuntimeSettings {
    pub output_folder: PathBuf,
    pub limits: OutputLimits,
    pub quiet_period: Duration,
}

/// The terminal runtime on `portable-pty`: one session for each agent that has
/// a running child.
pub struct PortableTerminalRuntime {
    settings: RuntimeSettings,
    sink: Arc<dyn TerminalEventSink>,
    sessions: Mutex<HashMap<AgentId, Arc<TerminalSession>>>,
    last_run: AtomicU64,
}

impl PortableTerminalRuntime {
    #[must_use]
    pub fn new(settings: RuntimeSettings, sink: Arc<dyn TerminalEventSink>) -> Self {
        Self {
            settings,
            sink,
            sessions: Mutex::new(HashMap::new()),
            last_run: AtomicU64::new(0),
        }
    }

    fn sessions(&self) -> MutexGuard<'_, HashMap<AgentId, Arc<TerminalSession>>> {
        self.sessions.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn running_session(&self, agent_id: AgentId) -> Option<Arc<TerminalSession>> {
        self.sessions()
            .get(&agent_id)
            .filter(|session| !session.has_exited())
            .cloned()
    }

    fn require_session(
        &self,
        agent_id: AgentId,
    ) -> Result<Arc<TerminalSession>, TerminalRuntimeError> {
        self.running_session(agent_id)
            .ok_or(TerminalRuntimeError::NotRunning(agent_id))
    }

    /// The snapshot of a session that is over, rebuilt from its output file.
    fn saved_snapshot(
        &self,
        agent_id: AgentId,
        saved_output: Option<&OutputKey>,
        size: TerminalSize,
    ) -> Result<Attachment, TerminalRuntimeError> {
        let key = saved_output.ok_or(TerminalRuntimeError::NoOutput(agent_id))?;
        let bytes = OutputStore::read_saved(&self.settings.output_folder, key)
            .map_err(|error| TerminalRuntimeError::Io(error.to_string()))?
            .ok_or(TerminalRuntimeError::NoOutput(agent_id))?;
        let mut emulator = ScreenEmulator::new(size, self.settings.limits.max_lines());
        // Nobody is there to read an answer to a query in old output.
        let _ = emulator.process(&bytes);
        Ok(Attachment {
            snapshot: emulator.snapshot(),
            subscription: None,
        })
    }
}

impl TerminalRuntime for PortableTerminalRuntime {
    fn spawn(&self, request: &SpawnTerminalRequest) -> Result<(), TerminalRuntimeError> {
        // Every caller passes here, so the rules for batch scripts cannot be
        // skipped by a caller that forgets them.
        validate_spawn_terminal(request)?;
        let mut sessions = self.sessions();
        if sessions
            .get(&request.agent_id)
            .is_some_and(|session| !session.has_exited())
        {
            return Err(TerminalRuntimeError::AlreadyRunning(request.agent_id));
        }
        let mut pty = spawn_in_pty(request)?;
        // The file of the last session stays until the new child has started.
        let store = match OutputStore::create(
            &self.settings.output_folder,
            &request.output_key,
            self.settings.limits,
        ) {
            Ok(store) => store,
            Err(error) => {
                // A child that nobody records must not keep running unseen.
                let _ = pty.child.kill();
                // Reaping ends the killed child, so it does not stay a zombie.
                let _ = pty.child.wait();
                return Err(TerminalRuntimeError::Io(error.to_string()));
            }
        };
        let session = TerminalSession::start(SessionParts {
            agent_id: request.agent_id,
            pty,
            emulator: ScreenEmulator::new(request.size, self.settings.limits.max_lines()),
            store,
            size: request.size,
            quiet_period: self.settings.quiet_period,
            sink: Arc::clone(&self.sink),
            run: RunId(self.last_run.fetch_add(1, Ordering::Relaxed) + 1),
        });
        sessions.insert(request.agent_id, session);
        Ok(())
    }

    fn attach(
        &self,
        agent_id: AgentId,
        saved_output: Option<&OutputKey>,
        size: TerminalSize,
        subscriber: Arc<dyn OutputSubscriber>,
    ) -> Result<Attachment, TerminalRuntimeError> {
        match self.running_session(agent_id) {
            Some(session) => session.attach(size, subscriber),
            None => self.saved_snapshot(agent_id, saved_output, size),
        }
    }

    fn detach(&self, agent_id: AgentId, subscription: SubscriptionId) {
        if let Some(session) = self.running_session(agent_id) {
            session.detach(subscription);
        }
    }

    fn write_input(&self, agent_id: AgentId, bytes: &[u8]) -> Result<(), TerminalRuntimeError> {
        self.require_session(agent_id)?.write_input(bytes)
    }

    fn paste(&self, agent_id: AgentId, text: &str) -> Result<(), TerminalRuntimeError> {
        self.require_session(agent_id)?.paste(text)
    }

    fn resize(&self, agent_id: AgentId, size: TerminalSize) -> Result<bool, TerminalRuntimeError> {
        self.require_session(agent_id)?.resize(size)
    }

    fn stop(&self, agent_id: AgentId, grace: Duration) -> Result<(), TerminalRuntimeError> {
        self.require_session(agent_id)?.stop(grace)
    }

    fn is_running(&self, agent_id: AgentId) -> bool {
        self.running_session(agent_id).is_some()
    }

    fn current_run(&self, agent_id: AgentId) -> Option<RunId> {
        self.sessions().get(&agent_id).map(|session| session.run())
    }

    fn quiet_for(&self, agent_id: AgentId) -> Option<Duration> {
        self.running_session(agent_id)
            .map(|session| session.quiet_for())
    }
}
