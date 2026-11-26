// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::mpsc;
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

/// One worker thread that stays alive and runs every closure it is sent: the
/// shape of the blocking pool the server runs each request on, where the
/// thread that served one request serves the next one too.
fn reused_worker() -> (mpsc::Sender<Thunk>, thread::JoinHandle<()>) {
    let (sender, receiver) = mpsc::channel::<Thunk>();
    let handle = thread::spawn(move || {
        while let Ok(work) = receiver.recv() {
            work();
        }
    });
    (sender, handle)
}

/// A closure the worker runs, so two jobs can carry different captured ids.
type Thunk = Box<dyn FnOnce() + Send>;

#[test]
fn a_dropped_scope_leaves_no_actor_on_a_reused_thread() {
    let agent_id = AgentId::generate();
    let (sender, handle) = reused_worker();

    sender
        .send(Box::new(move || {
            let _scope = ActorScope::enter(Actor::Agent(agent_id));
            assert_eq!(ActorScope::current(), Actor::Agent(agent_id));
        }))
        .expect("first job");
    // The first job ended before the second runs, so its guard is dropped on
    // this same pool thread, the way one request follows another.
    sender
        .send(Box::new(|| {
            assert_eq!(ActorScope::current(), Actor::User);
        }))
        .expect("second job");
    drop(sender);

    handle.join().expect("worker");
}

#[test]
fn no_label_is_current_by_default_and_enter_clears_an_outer_label() {
    assert_eq!(ActorScope::label(), None);
    let _outer = ActorScope::enter_labeled(Actor::Office, "Nightly");
    assert_eq!(ActorScope::label().as_deref(), Some("Nightly"));

    let _inner = ActorScope::enter(Actor::User);

    assert_eq!(ActorScope::label(), None);
}

#[test]
fn nested_labeled_scopes_restore_the_previous_actor_and_label() {
    let outer_schedule = kernel::ScheduleId::generate();
    {
        let _outer = ActorScope::enter_labeled(Actor::Schedule(outer_schedule), "Outer");
        {
            let _inner = ActorScope::enter_labeled(Actor::Office, "Inner");
            assert_eq!(ActorScope::current(), Actor::Office);
            assert_eq!(ActorScope::label().as_deref(), Some("Inner"));
        }
        assert_eq!(ActorScope::current(), Actor::Schedule(outer_schedule));
        assert_eq!(ActorScope::label().as_deref(), Some("Outer"));
    }

    assert_eq!(ActorScope::current(), Actor::User);
    assert_eq!(ActorScope::label(), None);
}
