// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

use crate::entities::TeamId;

#[derive(Clone, Copy)]
struct Ids {
    ceo: AgentId,
    lead_a: AgentId,
    lead_b: AgentId,
    member: AgentId,
    plain: AgentId,
}

fn fixed_ids() -> Ids {
    Ids {
        ceo: AgentId::from_u128(1),
        lead_a: AgentId::from_u128(2),
        lead_b: AgentId::from_u128(3),
        member: AgentId::from_u128(4),
        plain: AgentId::from_u128(5),
    }
}

/// The name a test row carries: the id text, since these rules do not read
/// names (only the CEO refusal does, and its test names its row itself).
fn name_of(agent_id: AgentId) -> String {
    agent_id.to_string()
}

fn tree() -> Vec<TreeRow> {
    let ids = fixed_ids();

    vec![
        TreeRow {
            agent_id: ids.ceo,
            name: name_of(ids.ceo),
            role: Some(Role::Ceo),
            parent_agent_id: None,
            team_id: None,
        },
        TreeRow {
            agent_id: ids.lead_a,
            name: name_of(ids.lead_a),
            role: Some(Role::Lead),
            parent_agent_id: Some(ids.ceo),
            team_id: Some(TeamId::from_u128(10)),
        },
        TreeRow {
            agent_id: ids.lead_b,
            name: name_of(ids.lead_b),
            role: Some(Role::Lead),
            parent_agent_id: Some(ids.ceo),
            team_id: Some(TeamId::from_u128(11)),
        },
        TreeRow {
            agent_id: ids.member,
            name: name_of(ids.member),
            role: Some(Role::Member),
            parent_agent_id: Some(ids.lead_a),
            team_id: Some(TeamId::from_u128(10)),
        },
        TreeRow {
            agent_id: ids.plain,
            name: name_of(ids.plain),
            role: None,
            parent_agent_id: None,
            team_id: None,
        },
    ]
}

fn placement(
    agent: AgentId,
    role: Role,
    parent: Option<AgentId>,
    team: Option<TeamId>,
) -> Placement {
    Placement {
        agent_id: agent,
        role,
        parent_agent_id: parent,
        team_id: team,
    }
}

#[test]
fn a_second_ceo_is_refused_and_names_the_first() {
    let ids = fixed_ids();
    let fresh = AgentId::from_u128(9);
    assert_eq!(
        check_placement(&tree(), placement(fresh, Role::Ceo, None, None)),
        Err(HierarchyError::SecondCeo {
            name: ids.ceo.to_string()
        })
    );
    // The ids.ceo itself keeps its role.
    assert_eq!(
        check_placement(&tree(), placement(ids.ceo, Role::Ceo, None, None)),
        Ok(())
    );
}

#[test]
fn a_second_lead_for_a_held_team_is_refused_and_names_the_seated_lead() {
    let ids = fixed_ids();
    let fresh = AgentId::from_u128(9);
    assert_eq!(
        check_placement(
            &tree(),
            placement(fresh, Role::Lead, None, Some(TeamId::from_u128(10))),
        ),
        Err(HierarchyError::SecondLead {
            name: name_of(ids.lead_a)
        })
    );
    // The seated lead itself keeps its own seat on a re-check.
    assert_eq!(
        check_placement(
            &tree(),
            placement(
                ids.lead_a,
                Role::Lead,
                Some(ids.ceo),
                Some(TeamId::from_u128(10))
            ),
        ),
        Ok(())
    );
}

#[test]
fn a_second_ceo_refusal_names_the_stored_ceo_not_its_id() {
    // The row carries the office's name for the CEO; the refusal shows that
    // name, not the id text (Efe, PR 36 r2 LOW). An empty name falls back
    // to the id, the only text the store guarantees.
    let ids = fixed_ids();
    let fresh = AgentId::from_u128(9);
    let mut rows = tree();
    rows[0].name = "Ada".to_owned();
    assert_eq!(
        check_placement(&rows, placement(fresh, Role::Ceo, None, None)),
        Err(HierarchyError::SecondCeo {
            name: "Ada".to_owned()
        })
    );
    rows[0].name = String::new();
    assert_eq!(
        check_placement(&rows, placement(fresh, Role::Ceo, None, None)),
        Err(HierarchyError::SecondCeo {
            name: ids.ceo.to_string()
        })
    );
}

#[test]
fn a_lead_sits_under_the_ceo_when_one_exists() {
    let ids = fixed_ids();
    let fresh = AgentId::from_u128(9);
    let team = TeamId::from_u128(12);
    assert_eq!(
        check_placement(&tree(), placement(fresh, Role::Lead, None, Some(team))),
        Ok(())
    );
    assert_eq!(
        resolved_parent(&tree(), placement(fresh, Role::Lead, None, Some(team))),
        Some(ids.ceo)
    );
    // A lead under a lead is refused.
    assert_eq!(
        check_placement(
            &tree(),
            placement(fresh, Role::Lead, Some(ids.lead_a), Some(team))
        ),
        Err(HierarchyError::RoleNotAllowed {
            child: Role::Lead,
            parent: Role::Ceo,
            allowed: "lead".to_owned()
        })
    );
}

#[test]
fn a_member_needs_a_lead_parent() {
    let ids = fixed_ids();
    let fresh = AgentId::from_u128(9);
    let team = TeamId::from_u128(10);
    assert_eq!(
        check_placement(
            &tree(),
            placement(fresh, Role::Member, Some(ids.lead_a), Some(team))
        ),
        Ok(())
    );
    assert_eq!(
        check_placement(
            &tree(),
            placement(fresh, Role::Member, Some(ids.ceo), Some(team))
        ),
        Err(HierarchyError::RoleNotAllowed {
            child: Role::Member,
            parent: Role::Lead,
            allowed: "member".to_owned()
        })
    );
    assert_eq!(
        check_placement(&tree(), placement(fresh, Role::Member, None, Some(team))),
        Err(HierarchyError::LeadMissing)
    );
    assert_eq!(
        check_placement(
            &tree(),
            placement(fresh, Role::Member, Some(fresh), Some(team))
        ),
        Err(HierarchyError::Cycle)
    );
    let unknown = AgentId::from_u128(99);
    assert_eq!(
        check_placement(
            &tree(),
            placement(fresh, Role::Member, Some(unknown), Some(team))
        ),
        Err(HierarchyError::ParentNotFound)
    );
}

#[test]
fn descendants_go_deepest_first_and_exclude_the_agent() {
    let ids = fixed_ids();
    assert_eq!(
        descendants_of(&tree(), ids.ceo),
        vec![ids.member, ids.lead_a, ids.lead_b]
    );
    assert_eq!(descendants_of(&tree(), ids.lead_a), vec![ids.member]);
    assert_eq!(
        descendants_of(&tree(), ids.member),
        [] as [kernel::AgentId; 0]
    );
}

#[test]
fn descendant_checks_follow_the_parent_chain() {
    let ids = fixed_ids();
    assert!(is_descendant_of(&tree(), ids.member, ids.ceo));
    assert!(is_descendant_of(&tree(), ids.member, ids.lead_a));
    assert!(!is_descendant_of(&tree(), ids.member, ids.lead_b));
    assert!(!is_descendant_of(&tree(), ids.lead_a, ids.member));
}

#[test]
fn a_generated_tree_never_loops_or_overflows() {
    let _ids = fixed_ids();
    // Property-style: a forest built only through check_placement keeps its
    // invariants. Build a 30-agent company and walk every parent chain.
    let new_id = |next: &mut u128| {
        *next += 1;
        AgentId::from_u128(*next)
    };
    let mut built: Vec<TreeRow> = Vec::new();
    let mut next = 100_u128;
    let ceo = new_id(&mut next);
    built.push(TreeRow {
        agent_id: ceo,
        name: name_of(ceo),
        role: Some(Role::Ceo),
        parent_agent_id: None,
        team_id: None,
    });
    for _ in 0..2 {
        let lead = new_id(&mut next);
        let team = TeamId::from_u128(next);
        check_placement(&built, placement(lead, Role::Lead, None, Some(team))).expect("lead ok");
        built.push(TreeRow {
            agent_id: lead,
            name: name_of(lead),
            role: Some(Role::Lead),
            parent_agent_id: resolved_parent(&built, placement(lead, Role::Lead, None, Some(team))),
            team_id: Some(team),
        });
        for _ in 0..12 {
            let member = new_id(&mut next);
            check_placement(
                &built,
                placement(member, Role::Member, Some(lead), Some(team)),
            )
            .expect("member ok");
            built.push(TreeRow {
                agent_id: member,
                name: name_of(member),
                role: Some(Role::Member),
                parent_agent_id: Some(lead),
                team_id: Some(team),
            });
        }
    }
    // Every chain reaches the ids.ceo in at most 2 steps and never loops.
    for row in &built {
        let mut steps = 0;
        let mut current = row.clone();
        while let Some(parent) = current.parent_agent_id {
            steps += 1;
            assert!(steps <= 2, "chain too deep");
            current = built
                .iter()
                .find(|candidate| candidate.agent_id == parent)
                .cloned()
                .expect("parent exists");
        }
        assert_eq!(current.role, Some(Role::Ceo));
    }
    assert_eq!(descendants_of(&built, ceo).len(), 26);
}

#[test]
fn a_looped_tree_terminates_instead_of_hanging() {
    // Hand-made rows whose parent pointers loop a > b > a. Only a corrupted
    // store can hold such rows, so the walkers only owe a termination: no
    // hang, no stack overflow.
    let a = AgentId::from_u128(200);
    let b = AgentId::from_u128(201);
    let looped = vec![
        TreeRow {
            agent_id: a,
            name: name_of(a),
            role: None,
            parent_agent_id: Some(b),
            team_id: None,
        },
        TreeRow {
            agent_id: b,
            name: name_of(b),
            role: None,
            parent_agent_id: Some(a),
            team_id: None,
        },
    ];
    let bound = std::time::Duration::from_secs(2);
    let started = std::time::Instant::now();
    // The walks read the rows as they are (b is a's child and a is b's
    // parent); the seen-set and the step cap only stop the loop from
    // extending the walk forever.
    assert_eq!(descendants_of(&looped, a), vec![b]);
    assert_eq!(descendants_of(&looped, b), vec![a]);
    assert!(is_descendant_of(&looped, a, b));
    assert!(is_descendant_of(&looped, b, a));
    // The negative query starts inside the cycle and chases a target that
    // no row holds, so neither a missing row nor a root can end the walk:
    // only the step cap turns it into a plain no. A walker that loops
    // forever instead of counting steps hangs here.
    let outside = AgentId::from_u128(202);
    assert!(!is_descendant_of(&looped, a, outside));
    assert!(
        started.elapsed() < bound,
        "the walks must end quickly, not spin"
    );
}
