// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::persistence::SCRIPTS;

fn repository() -> ActionRepository {
    let connection = Connection::open_in_memory().expect("open");
    for script in SCRIPTS {
        connection.execute_batch(script).expect("migrate");
    }
    ActionRepository::new(Arc::new(Mutex::new(connection)))
}

fn record(agent_id: AgentId, action: ActionKind, result: ActionResult) -> ActionRecord {
    ActionRecord {
        actor: Actor::User,
        target_agent_id: agent_id,
        action,
        result,
        at: Timestamp::from_unix_millis(1_700_000_000_000),
    }
}

#[test]
fn an_appended_entry_reads_back_with_actor_target_and_time() {
    let repository = repository();
    let agent_id = AgentId::generate();
    let record = record(agent_id, ActionKind::Create, ActionResult::Ok);

    let appended = repository.append(&record).expect("append");
    let listed = repository.list_for_agent(agent_id).expect("list");

    assert_eq!(listed, std::slice::from_ref(&appended));
    assert_eq!(appended.record, record);
}

#[test]
fn entries_come_oldest_first_and_only_for_the_target() {
    let repository = repository();
    let agent_id = AgentId::generate();
    let other = AgentId::generate();
    repository
        .append(&record(agent_id, ActionKind::Start, ActionResult::Ok))
        .expect("append");
    repository
        .append(&record(other, ActionKind::Start, ActionResult::Ok))
        .expect("append");
    repository
        .append(&record(
            agent_id,
            ActionKind::Stop,
            ActionResult::Failed("no such process".to_owned()),
        ))
        .expect("append");

    let listed = repository.list_for_agent(agent_id).expect("list");

    let actions: Vec<ActionKind> = listed.iter().map(|entry| entry.record.action).collect();
    assert_eq!(actions, [ActionKind::Start, ActionKind::Stop]);
    assert_eq!(
        listed[1].record.result,
        ActionResult::Failed("no such process".to_owned())
    );
}

#[test]
fn an_agent_actor_survives_the_round_trip() {
    let repository = repository();
    let agent_id = AgentId::generate();
    let actor = Actor::Agent(AgentId::generate());
    let mut entry = record(agent_id, ActionKind::NewTask, ActionResult::Ok);
    entry.actor = actor;

    repository.append(&entry).expect("append");

    assert_eq!(
        repository.list_for_agent(agent_id).expect("list")[0]
            .record
            .actor,
        actor
    );
}
