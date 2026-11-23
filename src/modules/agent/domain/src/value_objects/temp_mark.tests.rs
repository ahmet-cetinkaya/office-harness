// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn a_creator_round_trips_through_its_text() {
    let agent = Actor::Agent(AgentId::generate());
    let schedule = Actor::Schedule(kernel::ScheduleId::generate());
    for creator in [Actor::User, Actor::Office, agent, schedule] {
        assert_eq!(parse_creator(&creator_text(creator)), Some(creator));
    }
}

#[test]
fn the_user_and_an_agent_read_as_the_wire_names_them() {
    let agent_id = AgentId::generate();
    assert_eq!(creator_text(Actor::User), "user");
    assert_eq!(creator_text(Actor::Agent(agent_id)), agent_id.to_string());
}

#[test]
fn text_that_names_no_creator_is_refused() {
    assert_eq!(parse_creator("nobody"), None);
    assert_eq!(parse_creator(""), None);
}
