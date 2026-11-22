// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use hierarchy_application::queries::tree::RowPlace;
use hierarchy_domain::Role;
use kernel::AgentId;

use super::start_order;

fn placed(role: Role) -> RowPlace {
    RowPlace {
        role: Some(role),
        team_name: None,
    }
}

#[test]
fn members_start_before_their_lead_and_leads_before_the_ceo() {
    let [ceo, lead, first_member, second_member] = std::array::from_fn(|_| AgentId::generate());
    let places = HashMap::from([
        (ceo, placed(Role::Ceo)),
        (lead, placed(Role::Lead)),
        (first_member, placed(Role::Member)),
        (second_member, placed(Role::Member)),
    ]);

    let order = start_order(&[ceo, lead, first_member, second_member], &places);

    assert_eq!(order, [first_member, second_member, lead, ceo]);
}

#[test]
fn agents_outside_the_tree_start_with_the_members_in_list_order() {
    let [lead, outside, member] = std::array::from_fn(|_| AgentId::generate());
    let places = HashMap::from([(lead, placed(Role::Lead)), (member, placed(Role::Member))]);

    let order = start_order(&[lead, outside, member], &places);

    assert_eq!(order, [outside, member, lead]);
}
