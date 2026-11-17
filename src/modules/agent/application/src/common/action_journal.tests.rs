// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use super::*;
use kernel::{Actor, AgentId};

use crate::common::actor_scope::ActorScope;
use crate::test_support::{FixedClock, RecordingRecorder};

#[test]
fn the_entry_carries_the_actor_of_the_current_scope() {
    let recorder = Arc::new(RecordingRecorder::default());
    let journal = ActionJournal::new(recorder.clone(), Arc::new(FixedClock));
    let agent_id = AgentId::generate();

    let _scope = ActorScope::enter(Actor::Agent(agent_id));
    journal
        .record(agent_id, AgentAction::Create, None)
        .expect("record");

    let entries = recorder.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].actor, Actor::Agent(agent_id));
}
