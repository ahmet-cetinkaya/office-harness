// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn accepts_a_uuid() {
    assert!(OutputKey::parse("7d0c4a52-1f3e-4b7a-9c11-0a1b2c3d4e5f").is_ok());
}

#[test]
fn refuses_path_separators_and_dots() {
    for text in ["", "../x", "a/b", "a\\b", "a.log", &"x".repeat(65)] {
        assert!(OutputKey::parse(text).is_err(), "{text:?}");
    }
}
