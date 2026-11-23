// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn a_version_starts_at_one_and_grows_by_one() {
    assert_eq!(SecretVersion::first().number(), 1);
    assert_eq!(SecretVersion::first().next().number(), 2);
    assert!(SecretVersion::first() < SecretVersion::first().next());
}
