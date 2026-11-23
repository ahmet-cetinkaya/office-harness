// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex};

use audit_application::common::interfaces::ActionLog;
use audit_domain::{ActionEntry, ActionKind, ActionRecord, ActionResult};
use kernel::{Actor, Timestamp};
use serde_json::{Map, json};

use super::*;

#[derive(Default)]
struct MemoryLog(Mutex<Vec<ActionEntry>>);

impl ActionLog for MemoryLog {
    fn append(&self, record: &ActionRecord) -> Result<ActionEntry, AuditError> {
        let mut entries = self.0.lock().expect("lock");
        let entry = ActionEntry {
            id: 1,
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

fn request(fields: Value) -> Request {
    let Value::Object(fields) = fields else {
        panic!("an object")
    };
    Request {
        id: 4,
        kind: ACTIONS_REQUEST.to_owned(),
        fields: Map::from_iter(fields),
    }
}

#[test]
fn the_entries_of_an_agent_come_back_as_an_array() {
    let log = Arc::new(MemoryLog::default());
    let agent_id = AgentId::generate();
    log.append(&ActionRecord {
        actor: Actor::User,
        target_agent_id: Some(agent_id),
        action: ActionKind::Create,
        result: ActionResult::Ok,
        at: Timestamp::from_unix_millis(5),
        detail: None,
        actor_label: None,
    })
    .expect("append");
    let handlers = AuditHandlers::new(ListActionsHandler::new(log));

    let (response, disk_failure) =
        handlers.handle(&request(json!({"agent_id": agent_id.to_string()})));

    let Response::Success { id, result } = response else {
        panic!("success")
    };
    assert_eq!(id, 4);
    assert_eq!(result[0]["action"], "create");
    assert_eq!(disk_failure, None);
}

#[test]
fn a_missing_or_bad_agent_id_is_invalid_and_names_the_field() {
    let handlers = AuditHandlers::new(ListActionsHandler::new(Arc::new(MemoryLog::default())));

    for body in [
        json!({}),
        json!({"agent_id": "nope"}),
        json!({"agent_id": 5}),
    ] {
        let (response, _) = handlers.handle(&request(body));
        let Response::Failure { code, message, .. } = response else {
            panic!("failure")
        };
        assert_eq!(code, ErrorCode::Invalid);
        assert!(message.contains("agent_id"), "{message}");
    }
}
