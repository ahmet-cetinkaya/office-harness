// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex};

use hierarchy_application::common::interfaces::HoldStore;
use hierarchy_domain::{Hold, HoldScope, HoldSetter};
use kernel::{AgentId, Timestamp};
use rusqlite::Connection;

use super::hold_repository::HoldRepository;
use crate::persistence::migrations::SCRIPTS;

const FIRST_AGENT: u128 = 1;
const SECOND_AGENT: u128 = 2;

/// Runs the real migration only on a new file, as the host does.
fn open(path: &std::path::Path) -> HoldRepository {
    let is_new = !path.exists();
    let connection = Connection::open(path).expect("database");
    if is_new {
        connection.execute_batch(&SCRIPTS.concat()).expect("schema");
    }
    HoldRepository::new(Arc::new(Mutex::new(connection)))
}

fn agent_hold(agent: u128, set_by: HoldSetter, reason: Option<&str>) -> Hold {
    Hold {
        scope: HoldScope::Agent(AgentId::from_u128(agent)),
        set_by,
        since: Timestamp::from_unix_millis(1_700_000_000_000),
        reason: reason.map(str::to_owned),
    }
}

fn office_hold() -> Hold {
    Hold {
        scope: HoldScope::Office,
        set_by: HoldSetter::User,
        since: Timestamp::from_unix_millis(1_700_000_000_001),
        reason: None,
    }
}

#[test]
fn holds_survive_closing_and_reopening_the_database() {
    let directory = tempfile::tempdir().expect("directory");
    let path = directory.path().join("office.db");
    // Listed in `since` order, which is the order the store loads them in.
    let held = [
        agent_hold(FIRST_AGENT, HoldSetter::Office, Some("limit reached")),
        office_hold(),
    ];
    {
        let repository = open(&path);
        for hold in &held {
            repository.save_hold(hold).expect("saved");
        }
    }
    assert_eq!(open(&path).load_holds().expect("loaded"), held);
}

#[test]
fn saving_a_scope_twice_keeps_one_row() {
    let directory = tempfile::tempdir().expect("directory");
    let repository = open(&directory.path().join("office.db"));
    repository
        .save_hold(&agent_hold(FIRST_AGENT, HoldSetter::User, None))
        .expect("first");
    repository
        .save_hold(&agent_hold(FIRST_AGENT, HoldSetter::Office, Some("again")))
        .expect("second");
    assert_eq!(repository.load_holds().expect("loaded").len(), 1);
}

#[test]
fn delete_hold_removes_only_that_scope() {
    let directory = tempfile::tempdir().expect("directory");
    let repository = open(&directory.path().join("office.db"));
    repository.save_hold(&office_hold()).expect("office");
    repository
        .save_hold(&agent_hold(FIRST_AGENT, HoldSetter::User, None))
        .expect("agent");
    repository.delete_hold(HoldScope::Office).expect("deleted");
    assert_eq!(
        repository.load_holds().expect("loaded"),
        [agent_hold(FIRST_AGENT, HoldSetter::User, None)]
    );
}

#[test]
fn delete_agent_holds_removes_the_named_agents_and_keeps_the_rest() {
    let directory = tempfile::tempdir().expect("directory");
    let repository = open(&directory.path().join("office.db"));
    repository.save_hold(&office_hold()).expect("office");
    repository
        .save_hold(&agent_hold(FIRST_AGENT, HoldSetter::User, None))
        .expect("first");
    repository
        .save_hold(&agent_hold(SECOND_AGENT, HoldSetter::User, None))
        .expect("second");
    repository
        .delete_agent_holds(&[AgentId::from_u128(FIRST_AGENT)])
        .expect("deleted");
    let mut scopes: Vec<_> = repository
        .load_holds()
        .expect("loaded")
        .into_iter()
        .map(|hold| hold.scope)
        .collect();
    scopes.sort_by_key(|scope| matches!(scope, HoldScope::Office));
    assert_eq!(
        scopes,
        [
            HoldScope::Agent(AgentId::from_u128(SECOND_AGENT)),
            HoldScope::Office
        ]
    );
}
