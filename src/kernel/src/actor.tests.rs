// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn older_variants_keep_their_wire_form() {
    let agent_id = AgentId::from_u128(7);
    assert_eq!(
        serde_json::to_string(&Actor::User).expect("json"),
        r#"{"kind":"user"}"#
    );
    assert_eq!(
        serde_json::to_string(&Actor::Agent(agent_id)).expect("json"),
        format!(r#"{{"kind":"agent","agent_id":"{agent_id}"}}"#)
    );
}

#[test]
fn schedule_actor_round_trips_through_json() {
    let actor = Actor::Schedule(ScheduleId::from_u128(9));
    let json = serde_json::to_string(&actor).expect("json");
    assert_eq!(serde_json::from_str::<Actor>(&json).expect("json"), actor);
}
