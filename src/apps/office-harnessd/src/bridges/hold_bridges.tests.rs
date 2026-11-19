// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, PoisonError};

use audit_application::common::errors::AuditError;
use audit_application::common::interfaces::ActionLog;
use audit_domain::ActionEntry;
use kernel::Timestamp;

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
    let folder = tempfile::tempdir().expect("folder");
    let config = crate::config::Config {
        data_folder: folder.path().join("office"),
    };
    let server_log = crate::bootstrap::open_log(&config).expect("log");
    let hub = EventHub::new();
    let bridge = HoldJournalBridge::new(
        RecordActionHandler::new(Arc::clone(log) as Arc<dyn ActionLog>),
        hub.clone(),
        StorageAlerts::new(server_log),
    );
    (bridge, hub)
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
