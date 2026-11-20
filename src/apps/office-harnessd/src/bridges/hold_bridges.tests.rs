// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, PoisonError};

use agent_application::common::errors::PortError;
use agent_application::common::interfaces::AgentStore;
use agent_domain::entities::Agent;
use agent_domain::enums::AgentStatus;
use audit_application::common::errors::AuditError;
use audit_application::common::interfaces::ActionLog;
use audit_domain::ActionEntry;
use kernel::Timestamp;
use notification_domain::{AttentionEvent, AttentionKind};

use super::*;

const SINCE: Timestamp = Timestamp::from_unix_millis(1_700_000_000_000);

#[derive(Default)]
struct MemoryLog(Mutex<Vec<ActionRecord>>);

impl MemoryLog {
    fn records(&self) -> Vec<ActionRecord> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl ActionLog for MemoryLog {
    fn append(&self, record: &ActionRecord) -> Result<ActionEntry, AuditError> {
        let mut records = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        records.push(record.clone());
        Ok(ActionEntry {
            id: i64::try_from(records.len()).expect("small"),
            record: record.clone(),
        })
    }

    fn list_for_agent(&self, _agent_id: AgentId) -> Result<Vec<ActionEntry>, AuditError> {
        Ok(Vec::new())
    }

    fn list_office_level(&self) -> Result<Vec<ActionEntry>, AuditError> {
        Ok(Vec::new())
    }
}

fn bridge_over(log: &Arc<MemoryLog>) -> (HoldJournalBridge, EventHub) {
    let (bridge, hub, _needs_you) = bridge_with_queue(log);
    (bridge, hub)
}

fn bridge_with_queue(log: &Arc<MemoryLog>) -> (HoldJournalBridge, EventHub, NeedsYouBridge) {
    let folder = tempfile::tempdir().expect("folder");
    let config = crate::config::Config {
        data_folder: folder.path().join("office"),
    };
    let server_log = crate::bootstrap::open_log(&config).expect("log");
    let hub = EventHub::new();
    let alerts = StorageAlerts::new(server_log.clone());
    let needs_you =
        NeedsYouBridge::in_memory(Arc::new(NoAgents), hub.clone(), alerts.clone(), server_log);
    let bridge = HoldJournalBridge::new(
        RecordActionHandler::new(Arc::clone(log) as Arc<dyn ActionLog>),
        needs_you.clone(),
        hub.clone(),
        alerts,
    );
    (bridge, hub, needs_you)
}

fn hold_of(scope: HoldScope, set_by: HoldSetter) -> Hold {
    Hold {
        scope,
        set_by,
        since: SINCE,
        reason: None,
    }
}

#[test]
fn a_user_hold_of_an_agent_is_recorded_with_the_user_as_actor_and_the_agent_as_target() {
    let log = Arc::new(MemoryLog::default());
    let (bridge, hub) = bridge_over(&log);
    let mut events = hub.subscribe();
    let agent = AgentId::generate();

    bridge.held(&hold_of(HoldScope::Agent(agent), HoldSetter::User));

    let records = log.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].actor, Actor::User);
    assert_eq!(records[0].action, ActionKind::Hold);
    assert_eq!(records[0].target_agent_id, Some(agent));
    assert_eq!(records[0].at, SINCE);
    assert!(
        events
            .try_recv()
            .expect("action.recorded")
            .contains("action.recorded")
    );
}

#[test]
fn the_office_hold_has_no_target() {
    let log = Arc::new(MemoryLog::default());
    let (bridge, _hub) = bridge_over(&log);

    bridge.held(&hold_of(HoldScope::Office, HoldSetter::User));

    assert_eq!(log.records()[0].target_agent_id, None);
}

#[test]
fn an_automatic_hold_is_recorded_with_the_office_as_actor() {
    let log = Arc::new(MemoryLog::default());
    let (bridge, _hub) = bridge_over(&log);
    let agent = AgentId::generate();

    bridge.held(&hold_of(HoldScope::Agent(agent), HoldSetter::Office));

    assert_eq!(log.records()[0].actor, Actor::Office);
    assert_eq!(log.records()[0].action, ActionKind::Hold);
}

#[test]
fn a_release_is_recorded_with_the_user_as_actor() {
    let log = Arc::new(MemoryLog::default());
    let (bridge, _hub) = bridge_over(&log);

    bridge.released(HoldScope::Office);

    let records = log.records();
    assert_eq!(records[0].actor, Actor::User);
    assert_eq!(records[0].action, ActionKind::Release);
    assert_eq!(records[0].target_agent_id, None);
}

/// An office without agent rows: the queue only needs names for its order.
struct NoAgents;

impl AgentStore for NoAgents {
    fn insert(&self, _agent: &Agent) -> Result<(), PortError> {
        Ok(())
    }

    fn find(&self, _id: AgentId) -> Result<Option<Agent>, PortError> {
        Ok(None)
    }

    fn list(&self) -> Result<Vec<Agent>, PortError> {
        Ok(Vec::new())
    }

    fn update_status(&self, _id: AgentId, _status: AgentStatus) -> Result<(), PortError> {
        Ok(())
    }

    fn delete(&self, _id: AgentId) -> Result<(), PortError> {
        Ok(())
    }
}

fn attention_of(kind: AttentionKind) -> AttentionEvent {
    AttentionEvent {
        kind,
        detail: None,
        at: SINCE,
    }
}

#[test]
fn the_office_hold_release_keeps_the_item_of_an_automatic_agent_hold() {
    let log = Arc::new(MemoryLog::default());
    let (bridge, _hub, needs_you) = bridge_with_queue(&log);
    let agent = AgentId::generate();
    needs_you.record(agent, attention_of(AttentionKind::AutoHold), None, || {});

    bridge.released(HoldScope::Office);

    assert_eq!(needs_you.items().len(), 1);
}

#[test]
fn releasing_the_agent_hold_clears_its_auto_hold_item_and_tells_the_clients() {
    let log = Arc::new(MemoryLog::default());
    let (bridge, hub, needs_you) = bridge_with_queue(&log);
    let agent = AgentId::generate();
    needs_you.record(agent, attention_of(AttentionKind::AutoHold), None, || {});
    let mut events = hub.subscribe();

    bridge.released(HoldScope::Agent(agent));

    assert_eq!(needs_you.items(), []);
    let frames: Vec<String> = std::iter::from_fn(|| events.try_recv().ok())
        .map(|frame| frame.to_string())
        .collect();
    assert!(
        frames
            .iter()
            .any(|frame| frame.contains("needs_you.changed"))
    );
}

#[test]
fn releasing_an_agent_hold_keeps_an_item_of_another_kind() {
    let log = Arc::new(MemoryLog::default());
    let (bridge, _hub, needs_you) = bridge_with_queue(&log);
    let agent = AgentId::generate();
    needs_you.record(
        agent,
        attention_of(AttentionKind::WaitingForInput),
        None,
        || {},
    );

    bridge.released(HoldScope::Agent(agent));

    assert_eq!(needs_you.items().len(), 1);
}
