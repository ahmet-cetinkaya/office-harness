// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use agent_infrastructure::persistence::{
    PersistenceError, SessionProcessRepository, SessionRepository, SettingsRepository,
};
use kernel::AgentId;

use super::leftover_sweep::{Leftover, LeftoverNotifier, LeftoverSweep};
use super::sweep_gate::{SweepGate, SweepState, sweep_event};
use crate::bridges::NotificationBridge;
use crate::server::EventHub;

impl LeftoverNotifier for NotificationBridge {
    fn leftover_kept(&self, agent_id: AgentId, reason: &str) {
        self.record_leftover_kept(agent_id, reason);
    }
}

/// The sweep of a server start (0017 D5): it knows the stored identities of
/// the `lost` sessions from the start, closes the gate when there are any,
/// and runs once after the server has bound its listener.
pub struct OrphanSweep {
    gate: SweepGate,
    sweep: LeftoverSweep,
    grace: Duration,
    pending: Mutex<Option<Vec<Leftover>>>,
}

impl OrphanSweep {
    /// Reads the open identities of the `lost` sessions. The gate is closed
    /// from here on when there is something to sweep, so no request gets
    /// ahead of the sweep.
    ///
    /// # Errors
    /// A storage failure of one of the reads.
    pub fn prepare(
        processes: &SessionProcessRepository,
        sessions: &SessionRepository,
        settings: &SettingsRepository,
        sweep: LeftoverSweep,
    ) -> Result<Self, PersistenceError> {
        let leftovers = owned_leftovers(processes, sessions, &sweep)?;
        let grace = Duration::from_millis(settings.load()?.stop_grace_ms);
        Ok(Self {
            gate: SweepGate::new(!leftovers.is_empty()),
            sweep,
            grace,
            pending: Mutex::new(Some(leftovers)),
        })
    }

    #[must_use]
    pub fn gate(&self) -> SweepGate {
        self.gate.clone()
    }

    /// Starts the sweep on its own task, once. No sweep runs when no `lost`
    /// session has an open identity.
    pub fn spawn(&self, events: &EventHub) {
        let taken = self
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        let Some(leftovers) = taken.filter(|leftovers| !leftovers.is_empty()) else {
            return;
        };
        let (sweep, gate, grace, events) = (
            self.sweep.clone(),
            self.gate.clone(),
            self.grace,
            events.clone(),
        );
        tokio::spawn(async move {
            sweep.log.info(&format!(
                "sweep started over {} stored processes",
                leftovers.len()
            ));
            events.publish(&sweep_event(SweepState::Running));
            sweep.run(leftovers, grace).await;
            gate.finish();
            events.publish(&sweep_event(SweepState::Done));
            sweep.log.info("sweep done");
        });
    }
}

/// Each open identity of a `lost` session with the agent of its session. An
/// identity whose session is gone cannot be told to an agent and is skipped.
fn owned_leftovers(
    processes: &SessionProcessRepository,
    sessions: &SessionRepository,
    sweep: &LeftoverSweep,
) -> Result<Vec<Leftover>, PersistenceError> {
    let mut leftovers = Vec::new();
    for stored in processes.open_for_lost_sessions()? {
        let agent = sessions
            .find(stored.session_id)?
            .map(|session| session.agent_id);
        match agent {
            Some(agent_id) => leftovers.push(Leftover { stored, agent_id }),
            None => sweep.log.error(&format!(
                "the session {} of a stored process is gone",
                stored.session_id
            )),
        }
    }
    Ok(leftovers)
}
