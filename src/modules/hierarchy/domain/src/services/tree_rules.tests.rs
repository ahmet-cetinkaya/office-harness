// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const CEO: AgentId = AgentId::from_u128(1);
const LEAD_A: AgentId = AgentId::from_u128(2);
const LEAD_B: AgentId = AgentId::from_u128(3);
const MEMBER: AgentId = AgentId::from_u128(4);
const PLAIN: AgentId = AgentId::from_u128(5);

fn tree() -> Vec<TreeRow> {
    vec![
        TreeRow { agent_id: CEO, role: Some(Role::Ceo), parent_agent_id: None, team_id: None },
        TreeRow { agent_id: LEAD_A, role: Some(Role::Lead), parent_agent_id: Some(CEO), team_id: Some(TeamId::from_u128(10)) },
        TreeRow { agent_id: LEAD_B, role: Some(Role::Lead), parent_agent_id: Some(CEO), team_id: Some(TeamId::from_u128(11)) },
        TreeRow { agent_id: MEMBER, role: Some(Role::Member), parent_agent_id: Some(LEAD_A), team_id: Some(TeamId::from_u128(10)) },
        TreeRow { agent_id: PLAIN, role: None, parent_agent_id: None, team_id: None },
    ]
}

fn placement(agent: AgentId, role: Role, parent: Option<AgentId>, team: Option<TeamId>) -> Placement {
    Placement { agent_id: agent, role, parent_agent_id: parent, team_id: team }
}

#[test]
fn a_second_ceo_is_refused_and_names_the_first() {
    let fresh = AgentId::from_u128(9);
    assert_eq!(
        check_placement(&tree(), placement(fresh, Role::Ceo, None, None)),
        Err(HierarchyError::SecondCeo { name: CEO.to_string() })
    );
    // The CEO itself keeps its role.
    assert_eq!(check_placement(&tree(), placement(CEO, Role::Ceo, None, None)), Ok(()));
}

#[test]
fn a_lead_sits_under_the_ceo_when_one_exists() {
    let fresh = AgentId::from_u128(9);
    let team = TeamId::from_u128(12);
    assert_eq!(check_placement(&tree(), placement(fresh, Role::Lead, None, Some(team))), Ok(()));
    assert_eq!(
        resolved_parent(&tree(), placement(fresh, Role::Lead, None, Some(team))),
        Some(CEO)
    );
    // A lead under a lead is refused.
    assert_eq!(
        check_placement(&tree(), placement(fresh, Role::Lead, Some(LEAD_A), Some(team))),
        Err(HierarchyError::RoleNotAllowed { child: Role::Lead, parent: Role::Ceo, allowed: "lead".to_owned() })
    );
}

#[test]
fn a_member_needs_a_lead_parent() {
    let fresh = AgentId::from_u128(9);
    let team = TeamId::from_u128(10);
    assert_eq!(
        check_placement(&tree(), placement(fresh, Role::Member, Some(LEAD_A), Some(team))),
        Ok(())
    );
    assert_eq!(
        check_placement(&tree(), placement(fresh, Role::Member, Some(CEO), Some(team))),
        Err(HierarchyError::RoleNotAllowed { child: Role::Member, parent: Role::Lead, allowed: "member".to_owned() })
    );
    assert_eq!(
        check_placement(&tree(), placement(fresh, Role::Member, None, Some(team))),
        Err(HierarchyError::LeadMissing)
    );
    assert_eq!(
        check_placement(&tree(), placement(fresh, Role::Member, Some(fresh), Some(team))),
        Err(HierarchyError::Cycle)
    );
    let unknown = AgentId::from_u128(99);
    assert_eq!(
        check_placement(&tree(), placement(fresh, Role::Member, Some(unknown), Some(team))),
        Err(HierarchyError::ParentNotFound)
    );
}

#[test]
fn descendants_go_deepest_first_and_exclude_the_agent() {
    assert_eq!(descendants_of(&tree(), CEO), vec![MEMBER, LEAD_A, LEAD_B]);
    assert_eq!(descendants_of(&tree(), LEAD_A), vec![MEMBER]);
    assert!(descendants_of(&tree(), MEMBER).is_empty());
}

#[test]
fn descendant_checks_follow_the_parent_chain() {
    assert!(is_descendant_of(&tree(), MEMBER, CEO));
    assert!(is_descendant_of(&tree(), MEMBER, LEAD_A));
    assert!(!is_descendant_of(&tree(), MEMBER, LEAD_B));
    assert!(!is_descendant_of(&tree(), LEAD_A, MEMBER));
}

#[test]
fn a_generated_tree_never_loops_or_overflows() {
    // Property-style: a forest built only through check_placement keeps its
    // invariants. Build a 30-agent company and walk every parent chain.
    let mut built: Vec<TreeRow> = Vec::new();
    let mut next = 100_u128;
    let mut new_id = || {
        next += 1;
        AgentId::from_u128(next)
    };
    let ceo = new_id();
    built.push(TreeRow { agent_id: ceo, role: Some(Role::Ceo), parent_agent_id: None, team_id: None });
    for _ in 0..2 {
        let lead = new_id();
        let team = TeamId::from_u128(next);
        check_placement(&built, placement(lead, Role::Lead, None, Some(team))).expect("lead ok");
        built.push(TreeRow { agent_id: lead, role: Some(Role::Lead), parent_agent_id: resolved_parent(&built, placement(lead, Role::Lead, None, Some(team))), team_id: Some(team) });
        for _ in 0..12 {
            let member = new_id();
            check_placement(&built, placement(member, Role::Member, Some(lead), Some(team))).expect("member ok");
            built.push(TreeRow { agent_id: member, role: Some(Role::Member), parent_agent_id: Some(lead), team_id: Some(team) });
        }
    }
    // Every chain reaches the CEO in at most 2 steps and never loops.
    for row in &built {
        let mut steps = 0;
        let mut current = *row;
        while let Some(parent) = current.parent_agent_id {
            steps += 1;
            assert!(steps <= 2, "chain too deep");
            current = built.iter().copied().find(|candidate| candidate.agent_id == parent).expect("parent exists");
        }
        assert_eq!(current.role, Some(Role::Ceo));
    }
    assert_eq!(descendants_of(&built, ceo).len(), 26);
}
