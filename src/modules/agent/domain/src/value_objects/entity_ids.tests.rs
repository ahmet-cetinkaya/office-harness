// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn identifiers_round_trip_through_text() {
    let project = ProjectId::generate();
    let machine = MachineId::generate();
    assert_eq!(project.to_string().parse::<ProjectId>(), Ok(project));
    assert_eq!(machine.to_string().parse::<MachineId>(), Ok(machine));
}

#[test]
fn a_bad_identifier_names_its_field() {
    let error = "x".parse::<ProjectId>().expect_err("not a UUID");
    assert_eq!(error.to_string(), "project id is not a valid identifier: x");
}
