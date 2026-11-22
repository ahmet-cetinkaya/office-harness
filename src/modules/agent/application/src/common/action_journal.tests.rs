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

#[test]
fn a_detailed_entry_keeps_its_reason_and_a_plain_one_has_none() {
    let recorder = Arc::new(RecordingRecorder::default());
    let journal = ActionJournal::new(recorder.clone(), Arc::new(FixedClock));
    let agent_id = AgentId::generate();

    journal
        .record_detailed(agent_id, AgentAction::Start, None, Some("new session: why"))
        .expect("record");
    journal
        .record(agent_id, AgentAction::Stop, None)
        .expect("record");

    let entries = recorder.entries();
    assert_eq!(entries[0].detail.as_deref(), Some("new session: why"));
    assert_eq!(entries[1].detail, None);
}
