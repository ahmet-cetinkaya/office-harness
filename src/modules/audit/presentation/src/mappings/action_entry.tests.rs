// SPDX-License-Identifier: AGPL-3.0-or-later

use audit_domain::{ActionKind, ActionRecord, ActionResult};
use kernel::{AgentId, Timestamp};

use super::*;

fn entry() -> ActionEntry {
    ActionEntry {
        id: 7,
        record: ActionRecord {
            actor: Actor::User,
            target_agent_id: AgentId::generate(),
            action: ActionKind::Start,
            result: ActionResult::Failed("no such folder".to_owned()),
            at: Timestamp::from_unix_millis(1_700_000_000_000),
        },
    }
}

#[test]
fn an_entry_has_the_actor_target_action_result_and_time() {
    let entry = entry();

    let json = action_entry_json(&entry);

    assert_eq!(json["id"], 7);
    assert_eq!(json["actor"], "user");
    assert_eq!(json["agent_id"], entry.record.target_agent_id.to_string());
    assert_eq!(json["action"], "start");
    assert_eq!(json["result"], "no such folder");
    assert_eq!(json["at"], 1_700_000_000_000_i64);
}

#[test]
fn the_event_carries_the_entry() {
    let entry = entry();

    let frame: Value = serde_json::from_str(&action_recorded_event(&entry).encode()).expect("json");

    assert_eq!(frame["type"], "action.recorded");
    assert_eq!(frame["entry"], action_entry_json(&entry));
}
