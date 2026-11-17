// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn a_name_trims_and_lowers_for_its_key() {
    let name = TeamName::parse("  Backend ").expect("name");
    assert_eq!(name.display(), "Backend");
    assert_eq!(name.key(), "backend");
}

#[test]
fn an_empty_or_oversize_name_is_refused() {
    assert_eq!(
        TeamName::parse("   "),
        Err(HierarchyError::Empty { field: "team name" })
    );
    let long = "x".repeat(65);
    assert_eq!(
        TeamName::parse(&long),
        Err(HierarchyError::TooLong {
            field: "team name",
            max: 64
        })
    );
}
