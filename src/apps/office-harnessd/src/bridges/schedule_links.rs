// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use hierarchy_application::common::ports::{ScheduledOutcome, ScheduledOutcomes};
use kernel::ScheduleId;

/// The name that each schedule had at its last run. The action record names a
/// schedule by this label also after a rename or a delete (0025 D7), and the
/// queue, which knows only the schedule id, reads it for its own entries.
#[derive(Clone, Default)]
pub struct ScheduleLabels {
    names: Arc<Mutex<HashMap<ScheduleId, Arc<str>>>>,
}

impl ScheduleLabels {
    pub fn remember(&self, schedule: ScheduleId, name: &str) {
        self.names
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(schedule, Arc::from(name));
    }

    #[must_use]
    pub fn get(&self, schedule: ScheduleId) -> Option<Arc<str>> {
        self.names
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&schedule)
            .cloned()
    }
}

/// The queue's `ScheduledOutcomes` port, connected after the services that
/// build the queue: the schedule bridge needs the queue, and the queue needs
/// the bridge, so the bridge joins last.
#[derive(Clone, Default)]
pub struct ScheduledOutcomeRelay {
    target: Arc<OnceLock<Arc<dyn ScheduledOutcomes>>>,
}

impl ScheduledOutcomeRelay {
    pub fn connect(&self, outcomes: Arc<dyn ScheduledOutcomes>) {
        // The server connects once; a second call has nothing to change.
        let _ = self.target.set(outcomes);
    }
}

impl ScheduledOutcomes for ScheduledOutcomeRelay {
    fn settled(&self, run_id: i64, outcome: ScheduledOutcome) {
        if let Some(target) = self.target.get() {
            target.settled(run_id, outcome);
        }
    }
}
