// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

use crate::value_objects::Role;

const CEO: u128 = 1;
const LEAD: u128 = 2;
const MEMBER: u128 = 3;
const OTHER_LEAD: u128 = 4;

fn id(value: u128) -> AgentId {
    AgentId::from_u128(value)
}

fn row(agent: u128, parent: Option<u128>) -> TreeRow {
    TreeRow {
        agent_id: id(agent),
        name: agent.to_string(),
        role: Some(Role::Member),
        parent_agent_id: parent.map(id),
        team_id: None,
    }
}

fn rows() -> Vec<TreeRow> {
    vec![
        row(CEO, None),
        row(LEAD, Some(CEO)),
        row(MEMBER, Some(LEAD)),
        row(OTHER_LEAD, Some(CEO)),
    ]
}

fn hold_on(scope: HoldScope) -> Hold {
    Hold {
        scope,
        set_by: HoldSetter::User,
        since: Timestamp::from_unix_millis(0),
        reason: None,
    }
}

fn agent_hold(agent: u128) -> Hold {
    hold_on(HoldScope::Agent(id(agent)))
}

#[test]
fn no_hold_refuses_nothing() {
    assert_eq!(hold_refusal(&[], &rows(), id(MEMBER), Some(id(LEAD))), None);
}

#[test]
fn office_hold_wins_over_agent_holds() {
    let holds = [agent_hold(LEAD), hold_on(HoldScope::Office)];
    let refusal = hold_refusal(&holds, &rows(), id(MEMBER), None);
    assert_eq!(refusal, Some(HoldRefusal::Office));
    assert_eq!(refusal.unwrap().to_string(), "the office is on hold");
}

#[test]
fn agent_hold_refuses_the_agent_and_its_descendant_as_caller() {
    let holds = [agent_hold(LEAD)];
    for caller in [LEAD, MEMBER] {
        assert_eq!(
            hold_refusal(&holds, &rows(), id(caller), None),
            Some(HoldRefusal::Agent(id(LEAD)))
        );
    }
}

#[test]
fn agent_hold_refuses_actions_on_the_agent_and_its_descendant() {
    let holds = [agent_hold(LEAD)];
    for target in [LEAD, MEMBER] {
        assert_eq!(
            hold_refusal(&holds, &rows(), id(OTHER_LEAD), Some(id(target))),
            Some(HoldRefusal::Agent(id(LEAD)))
        );
    }
}

#[test]
fn agent_hold_refuses_an_ancestor_acting_on_the_held_agent() {
    let refusal = hold_refusal(&[agent_hold(LEAD)], &rows(), id(CEO), Some(id(LEAD)));
    assert_eq!(refusal, Some(HoldRefusal::Agent(id(LEAD))));
    assert_eq!(
        refusal.unwrap().to_string(),
        format!("the agent {} is on hold", id(LEAD))
    );
}

#[test]
fn agent_hold_leaves_unrelated_agents_alone() {
    assert_eq!(
        hold_refusal(&[agent_hold(LEAD)], &rows(), id(CEO), Some(id(OTHER_LEAD))),
        None
    );
}

#[test]
fn an_agent_placed_under_a_held_lead_later_is_covered() {
    let holds = [agent_hold(LEAD)];
    assert_eq!(hold_refusal(&holds, &rows(), id(OTHER_LEAD), None), None);
    let mut moved = rows();
    moved[3].parent_agent_id = Some(id(LEAD));
    assert_eq!(
        hold_refusal(&holds, &moved, id(OTHER_LEAD), None),
        Some(HoldRefusal::Agent(id(LEAD)))
    );
}

#[test]
fn a_cycle_in_the_snapshot_ends_without_a_hold() {
    let cyclic = vec![row(LEAD, Some(MEMBER)), row(MEMBER, Some(LEAD))];
    assert_eq!(
        hold_refusal(&[agent_hold(CEO)], &cyclic, id(LEAD), Some(id(MEMBER))),
        None
    );
}
