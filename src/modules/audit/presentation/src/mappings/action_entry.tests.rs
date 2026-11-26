// SPDX-License-Identifier: AGPL-3.0-or-later

use audit_domain::{ActionKind, ActionRecord, ActionResult};
use kernel::{AgentId, Timestamp};

use super::*;

fn entry() -> ActionEntry {
    ActionEntry {
        id: 7,
        record: ActionRecord {
            actor: Actor::User,
            target_agent_id: Some(AgentId::generate()),
            action: ActionKind::Start,
            result: ActionResult::Failed("no such folder".to_owned()),
            at: Timestamp::from_unix_millis(1_700_000_000_000),
            detail: None,
            actor_label: None,
        },
    }
}

#[test]
fn an_entry_has_the_actor_target_action_result_and_time() {
    let entry = entry();

    let json = action_entry_json(&entry);

    assert_eq!(json["id"], 7);
    assert_eq!(json["actor"], "user");
    assert_eq!(
        json["agent_id"],
        entry.record.target_agent_id.expect("target").to_string()
    );
    assert_eq!(json["action"], "start");
    assert_eq!(json["result"], "no such folder");
    assert!(json["detail"].is_null());
    assert_eq!(json["at"], 1_700_000_000_000_i64);
}

#[test]
fn the_event_carries_the_entry() {
    let entry = entry();

    let frame: Value = serde_json::from_str(&action_recorded_event(&entry).encode()).expect("json");

    assert_eq!(frame["type"], "action.recorded");
    assert_eq!(frame["entry"], action_entry_json(&entry));
}

#[test]
fn an_office_level_entry_has_the_office_actor_and_a_null_agent() {
    let mut entry = entry();
    entry.record.actor = Actor::Office;
    entry.record.target_agent_id = None;
    entry.record.action = ActionKind::Hold;

    let json = action_entry_json(&entry);

    assert_eq!(json["actor"], "office");
    assert!(json["agent_id"].is_null());
    assert_eq!(json["action"], "hold");
}

#[test]
fn a_schedule_actor_is_written_as_schedule_and_its_id() {
    let schedule_id = kernel::ScheduleId::from_u128(3);
    let mut entry = entry();
    entry.record.actor = Actor::Schedule(schedule_id);

    assert_eq!(
        action_entry_json(&entry)["actor"],
        format!("schedule:{schedule_id}")
    );
}

#[test]
fn a_detail_is_written_as_text() {
    let mut entry = entry();
    entry.record.detail = Some("new session: no earlier session".to_owned());

    assert_eq!(
        action_entry_json(&entry)["detail"],
        "new session: no earlier session"
    );
}

#[test]
fn an_actor_label_is_emitted_only_when_the_entry_has_one() {
    let mut entry = entry();
    assert!(action_entry_json(&entry).get("actor_label").is_none());

    entry.record.actor_label = Some("Nightly report".to_owned());

    assert_eq!(action_entry_json(&entry)["actor_label"], "Nightly report");
}
