// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn agent(number: u128) -> AgentId {
    AgentId::from_u128(number)
}

fn store() -> MemoryHookSecretStore {
    MemoryHookSecretStore::new().expect("store")
}

#[test]
fn the_secret_of_a_session_authorizes_that_session() {
    let store = store();
    let secret = store.issue(agent(1), "s1").expect("issue");
    assert_eq!(
        store.authorize("s1", secret.expose_secret()),
        Some(agent(1))
    );
}

#[test]
fn a_wrong_secret_an_unknown_session_and_another_sessions_secret_all_give_none() {
    let store = store();
    let first = store.issue(agent(1), "s1").expect("issue");
    let second = store.issue(agent(2), "s2").expect("issue");
    assert_eq!(store.authorize("s1", "not the secret"), None);
    assert_eq!(store.authorize("s1", ""), None);
    assert_eq!(store.authorize("unknown", first.expose_secret()), None);
    assert_eq!(store.authorize("s1", second.expose_secret()), None);
}

#[test]
fn a_revoked_session_refuses_its_secret() {
    let store = store();
    let secret = store.issue(agent(1), "s1").expect("issue");
    store.revoke_session("s1");
    assert_eq!(store.authorize("s1", secret.expose_secret()), None);
}

#[test]
fn a_new_secret_for_the_same_session_replaces_the_old_one() {
    let store = store();
    let old = store.issue(agent(1), "s1").expect("issue");
    let new = store.issue(agent(1), "s1").expect("issue");
    assert_eq!(store.authorize("s1", old.expose_secret()), None);
    assert_eq!(store.authorize("s1", new.expose_secret()), Some(agent(1)));
}

#[test]
fn revoking_an_agent_drops_every_secret_of_it_and_no_other() {
    let store = store();
    let one = store.issue(agent(1), "s1").expect("issue");
    let other = store.issue(agent(2), "s2").expect("issue");
    store.revoke_agent(agent(1));
    assert_eq!(store.authorize("s1", one.expose_secret()), None);
    assert_eq!(store.authorize("s2", other.expose_secret()), Some(agent(2)));
}

#[test]
fn the_comparison_needs_equal_text() {
    assert!(same_text("abc", "abc"));
    assert!(!same_text("abc", "abd"));
    assert!(!same_text("abc", "ab"));
}
