// SPDX-License-Identifier: AGPL-3.0-or-later

use std::thread;

use super::*;
use kernel::{Actor, AgentId};

#[test]
fn the_default_actor_is_the_user() {
    assert_eq!(ActorScope::current(), Actor::User);
}

#[test]
fn an_entered_agent_actor_holds_until_the_guard_drops() {
    let agent_id = AgentId::generate();

    {
        let _scope = ActorScope::enter(Actor::Agent(agent_id));

        assert_eq!(ActorScope::current(), Actor::Agent(agent_id));
    }

    assert_eq!(ActorScope::current(), Actor::User);
}

#[test]
fn nested_scopes_restore_in_order() {
    let outer_agent = AgentId::generate();
    let inner_agent = AgentId::generate();

    {
        let _outer = ActorScope::enter(Actor::Agent(outer_agent));

        {
            let _inner = ActorScope::enter(Actor::Agent(inner_agent));

            assert_eq!(ActorScope::current(), Actor::Agent(inner_agent));
        }

        assert_eq!(ActorScope::current(), Actor::Agent(outer_agent));
    }

    assert_eq!(ActorScope::current(), Actor::User);
}

#[test]
fn a_scope_on_another_thread_does_not_leak() {
    let agent_id = AgentId::generate();

    thread::spawn(move || {
        let _scope = ActorScope::enter(Actor::Agent(agent_id));

        assert_eq!(ActorScope::current(), Actor::Agent(agent_id));
    })
    .join()
    .expect("thread");

    assert_eq!(ActorScope::current(), Actor::User);
}
