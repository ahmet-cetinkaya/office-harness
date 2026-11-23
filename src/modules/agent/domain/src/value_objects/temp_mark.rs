// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::{Actor, AgentId};

use super::Objective;

const USER: &str = "user";
const OFFICE: &str = "office";

/// What makes an agent a temp: the objective it was made for and who made it
/// (0020 D1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempMark {
    pub objective: Objective,
    pub creator: Actor,
}

/// The creator as text: `user`, `office`, or the agent id.
#[must_use]
pub fn creator_text(creator: Actor) -> String {
    match creator {
        Actor::User => USER.to_owned(),
        Actor::Office => OFFICE.to_owned(),
        Actor::Agent(agent_id) => agent_id.to_string(),
    }
}

/// The creator from its text; `None` for text that is none of the three.
#[must_use]
pub fn parse_creator(text: &str) -> Option<Actor> {
    match text {
        USER => Some(Actor::User),
        OFFICE => Some(Actor::Office),
        other => AgentId::from_uuid_text(other).map(Actor::Agent),
    }
}

#[cfg(test)]
#[path = "temp_mark.tests.rs"]
mod tests;
