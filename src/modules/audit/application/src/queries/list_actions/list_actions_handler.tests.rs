// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex};

use audit_domain::{ActionKind, ActionRecord, ActionResult};
use kernel::{Actor, Timestamp};

use super::*;
use crate::commands::record_action::RecordActionHandler;

#[derive(Default)]
struct MemoryLog(Mutex<Vec<ActionEntry>>);

impl ActionLog for MemoryLog {
    fn append(&self, record: &ActionRecord) -> Result<ActionEntry, AuditError> {
        let mut entries = self.0.lock().expect("lock");
        let entry = ActionEntry {
            id: i64::try_from(entries.len()).expect("small") + 1,
            record: record.clone(),
        };
        entries.push(entry.clone());
        Ok(entry)
    }

    fn list_for_agent(&self, agent_id: AgentId) -> Result<Vec<ActionEntry>, AuditError> {
        let entries = self.0.lock().expect("lock");
        Ok(entries
            .iter()
            .filter(|entry| entry.record.target_agent_id == Some(agent_id))
            .cloned()
            .collect())
    }

    fn list_office_level(&self) -> Result<Vec<ActionEntry>, AuditError> {
        let entries = self.0.lock().expect("lock");
        Ok(entries
            .iter()
            .filter(|entry| entry.record.target_agent_id.is_none())
            .rev()
            .cloned()
            .collect())
    }
}

fn record(agent_id: AgentId, action: ActionKind, at: i64) -> ActionRecord {
    ActionRecord {
        actor: Actor::User,
        target_agent_id: Some(agent_id),
        action,
        result: ActionResult::Ok,
        at: Timestamp::from_unix_millis(at),
        detail: None,
    }
}

#[test]
fn a_recorded_action_is_listed_for_its_agent_only() {
    let log = Arc::new(MemoryLog::default());
    let recorder = RecordActionHandler::new(log.clone());
    let lister = ListActionsHandler::new(log);
    let first = AgentId::generate();
    let second = AgentId::generate();

    recorder
        .handle(&record(first, ActionKind::Create, 10))
        .expect("record");
    recorder
        .handle(&record(second, ActionKind::Create, 11))
        .expect("record");
    recorder
        .handle(&record(first, ActionKind::Start, 12))
        .expect("record");

    let entries = lister.handle(first).expect("list");
    let actions: Vec<ActionKind> = entries.iter().map(|entry| entry.record.action).collect();
    assert_eq!(actions, [ActionKind::Create, ActionKind::Start]);
    assert!(
        entries
            .iter()
            .all(|entry| entry.record.actor == Actor::User)
    );
}
