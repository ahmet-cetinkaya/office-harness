// SPDX-License-Identifier: AGPL-3.0-or-later

use std::cell::RefCell;

use agent_application::common::errors::PortError;
use agent_application::common::interfaces::ActionRecorder;
use agent_application::common::models::{AgentAction, AgentActionEntry};
use audit_application::commands::record_action::RecordActionHandler;
use audit_domain::{ActionKind, ActionRecord, ActionResult};
use audit_presentation::action_recorded_event;
use wire::Event;

use crate::bootstrap::StorageAlerts;

thread_local! {
    /// The `action.recorded` events of the request that runs on this thread.
    static RECORDED: RefCell<Vec<Event>> = const { RefCell::new(Vec::new()) };
}

/// Takes the `action.recorded` events that the calling thread's request
/// caused. The request gateway publishes them after the events of the change
/// itself, so a client sees the change before its record.
#[must_use]
pub fn take_recorded_events() -> Vec<Event> {
    RECORDED.with(|recorded| std::mem::take(&mut *recorded.borrow_mut()))
}

/// The agent module's `ActionRecorder`, backed by the audit module. The change
/// that an entry describes is already saved when the entry is written, so a
/// failed write must not turn that success into an error: the bridge raises
/// `storage.error`, logs it, and reports success.
pub struct ActionRecorderBridge {
    record: RecordActionHandler,
    alerts: StorageAlerts,
}

impl ActionRecorderBridge {
    #[must_use]
    pub fn new(record: RecordActionHandler, alerts: StorageAlerts) -> Self {
        Self { record, alerts }
    }
}

impl ActionRecorder for ActionRecorderBridge {
    fn record(&self, entry: &AgentActionEntry) -> Result<(), PortError> {
        let stored = self.record.handle(&ActionRecord {
            actor: entry.actor,
            target_agent_id: entry.agent_id,
            action: action_kind(entry.action),
            result: entry
                .failure
                .clone()
                .map_or(ActionResult::Ok, ActionResult::Failed),
            at: entry.at,
        });
        match stored {
            Ok(stored) => RECORDED.with(|recorded| {
                recorded.borrow_mut().push(action_recorded_event(&stored));
            }),
            Err(error) => self
                .alerts
                .report_disk_failure(&format!("the action record failed: {error}")),
        }
        Ok(())
    }
}

fn action_kind(action: AgentAction) -> ActionKind {
    match action {
        AgentAction::Create => ActionKind::Create,
        AgentAction::Start => ActionKind::Start,
        AgentAction::Stop => ActionKind::Stop,
        AgentAction::Delete => ActionKind::Delete,
        AgentAction::NewTask => ActionKind::NewTask,
        AgentAction::Continue => ActionKind::Continue,
        AgentAction::TeamCreate => ActionKind::TeamCreate,
        AgentAction::TeamDelete => ActionKind::TeamDelete,
        AgentAction::Directive => ActionKind::Directive,
        AgentAction::Refused => ActionKind::Refused,
    }
}
