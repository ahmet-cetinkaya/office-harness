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
        target_agent_id: Some(agent_id),
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

fn office_record(action: ActionKind, at: i64) -> ActionRecord {
    ActionRecord {
        actor: Actor::Office,
        target_agent_id: None,
        action,
        result: ActionResult::Ok,
        at: Timestamp::from_unix_millis(at),
    }
}

#[test]
fn an_office_actor_and_a_missing_target_survive_the_round_trip() {
    let repository = repository();
    let appended = repository
        .append(&office_record(ActionKind::Hold, 1))
        .expect("append");

    let listed = repository.list_office_level().expect("list");

    assert_eq!(listed, [appended]);
    assert_eq!(listed[0].record.actor, Actor::Office);
    assert_eq!(listed[0].record.target_agent_id, None);
}

#[test]
fn office_level_entries_come_newest_first_without_agent_entries() {
    let repository = repository();
    repository
        .append(&office_record(ActionKind::Hold, 1))
        .expect("append");
    repository
        .append(&record(
            AgentId::generate(),
            ActionKind::Start,
            ActionResult::Ok,
        ))
        .expect("append");
    repository
        .append(&office_record(ActionKind::Release, 2))
        .expect("append");

    let listed = repository.list_office_level().expect("list");

    let actions: Vec<ActionKind> = listed.iter().map(|entry| entry.record.action).collect();
    assert_eq!(actions, [ActionKind::Release, ActionKind::Hold]);
}

#[test]
fn the_office_level_list_stops_at_500_entries() {
    let repository = repository();
    for at in 0..501 {
        repository
            .append(&office_record(ActionKind::Hold, at))
            .expect("append");
    }

    let listed = repository.list_office_level().expect("list");

    assert_eq!(listed.len(), 500);
    assert_eq!(listed[0].id, 501);
}

#[test]
fn the_nullable_target_migration_keeps_every_row() {
    let connection = Connection::open_in_memory().expect("open");
    connection.execute_batch(SCRIPTS[0]).expect("0001");
    let agent_id = AgentId::generate();
    for (actor, action, result, at) in [("user", "start", "ok", 5), ("agent:x", "stop", "boom", 6)]
    {
        connection
            .execute(
                "INSERT INTO audit_actions (actor, target_agent_id, action, result, at) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![actor, agent_id.to_string(), action, result, at],
            )
            .expect("insert");
    }

    connection.execute_batch(SCRIPTS[1]).expect("0002");

    let rows: Vec<(i64, String, String, String, String, i64)> = connection
        .prepare(
            "SELECT id, actor, target_agent_id, action, result, at FROM audit_actions ORDER BY id",
        )
        .expect("prepare")
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .expect("query")
        .collect::<rusqlite::Result<_>>()
        .expect("rows");
    let target = agent_id.to_string();
    assert_eq!(
        rows,
        [
            (
                1,
                "user".to_owned(),
                target.clone(),
                "start".to_owned(),
                "ok".to_owned(),
                5
            ),
            (
                2,
                "agent:x".to_owned(),
                target,
                "stop".to_owned(),
                "boom".to_owned(),
                6
            ),
        ]
    );
    let repository = ActionRepository::new(Arc::new(Mutex::new(connection)));
    let appended = repository
        .append(&office_record(ActionKind::Hold, 7))
        .expect("a None target is accepted");
    // Ids keep counting after the rebuild: a new entry never reuses a copied id.
    assert_eq!(appended.id, 3);
    assert_eq!(repository.list_office_level().expect("list").len(), 1);
}
