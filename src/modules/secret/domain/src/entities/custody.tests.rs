// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn a_custody_round_trips_through_its_text() {
    for custody in [Custody::OsKeyring, Custody::KeyFile] {
        assert_eq!(Custody::parse(custody.as_str()), Some(custody));
    }
    assert_eq!(Custody::parse("cloud"), None);
}
