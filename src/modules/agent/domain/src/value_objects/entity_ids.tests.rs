// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn identifiers_round_trip_through_text() {
    let machine = MachineId::generate();
    assert_eq!(machine.to_string().parse::<MachineId>(), Ok(machine));
}
