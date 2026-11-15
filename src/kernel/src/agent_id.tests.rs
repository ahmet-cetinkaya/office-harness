// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn round_trips_through_text() {
    let agent_id = AgentId::generate();
    assert_eq!(agent_id.to_string().parse::<AgentId>(), Ok(agent_id));
}

#[test]
fn rejects_text_that_is_not_a_uuid() {
    assert_eq!(
        "not-an-id".parse::<AgentId>(),
        Err(AgentIdError("not-an-id".to_owned()))
    );
}
