// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn the_wire_names_round_trip() {
    for role in [Role::Ceo, Role::Lead, Role::Member] {
        assert_eq!(Role::from_wire(role.as_str()), Some(role));
    }
    assert_eq!(Role::from_wire("boss"), None);
    assert!(parse_role("role", "member").is_ok());
    assert_eq!(
        parse_role("role", "boss"),
        Err(HierarchyError::UnknownRole {
            field: "role",
            text: "boss".to_owned()
        })
    );
}

#[test]
fn each_role_allows_only_its_children() {
    assert_eq!(Role::Ceo.allowed_child_roles(), &[Role::Lead]);
    assert_eq!(Role::Lead.allowed_child_roles(), &[Role::Member]);
    assert!(Role::Member.allowed_child_roles().is_empty());
}
