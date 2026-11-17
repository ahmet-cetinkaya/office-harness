// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn agent(number: u128) -> AgentId {
    AgentId::from_u128(number)
}

#[test]
fn a_new_session_replaces_the_old_credential() {
    let store = MemoryAgentCallStore::new();
    let first = store.issue(agent(1), "s1").expect("issue");
    let second = store.issue(agent(1), "s2").expect("issue");
    assert_ne!(first.token, second.token);
    let current = store.current(agent(1)).expect("credential");
    assert_eq!(current.session, "s2");
    assert_eq!(current.token, second.token);
}

#[test]
fn revoking_an_old_session_leaves_the_newer_credential() {
    let store = MemoryAgentCallStore::new();
    store.issue(agent(1), "s1").expect("issue");
    store.issue(agent(1), "s2").expect("issue");
    store.revoke_session(agent(1), "s1");
    assert!(store.current(agent(1)).is_some());
    store.revoke_session(agent(1), "s2");
    assert!(store.current(agent(1)).is_none());
}

#[test]
fn an_agent_never_sees_the_credential_of_another() {
    let store = MemoryAgentCallStore::new();
    let one = store.issue(agent(1), "s1").expect("issue");
    let two = store.issue(agent(2), "s1").expect("issue");
    assert_ne!(one.token, two.token);
    store.revoke_agent(agent(1));
    assert!(store.current(agent(1)).is_none());
    assert!(store.current(agent(2)).is_some());
}

#[test]
fn the_debug_text_hides_the_token() {
    let store = MemoryAgentCallStore::new();
    let credential = store.issue(agent(1), "s1").expect("issue");
    let text = format!("{credential:?}");
    assert!(!text.contains(credential.token.expose_secret()));
}
